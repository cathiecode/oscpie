use std::{
    collections::{HashMap, HashSet},
    fmt::Debug,
    hash::Hash,
};

pub trait Versioned<V> {
    fn version(&self) -> V;
}

pub struct Migrator<T, V> {
    edge: (V, V),
    migrate: Box<dyn Fn(T) -> Result<T, String>>,
}

impl<T, V> Migrator<T, V>
where
    T: Versioned<V>,
    V: Eq + Hash + Clone + Debug,
{
    pub fn new(from: V, to: V, migrate: impl Fn(T) -> Result<T, String> + 'static) -> Self {
        Migrator {
            edge: (from, to),
            migrate: Box::new(migrate),
        }
    }
}

pub struct CompositMigrator<T, V> {
    migrators: HashMap<(V, V), Migrator<T, V>>,
}

impl<T, V> CompositMigrator<T, V>
where
    T: Versioned<V>,
    V: Eq + Hash + Clone + Debug,
{
    pub fn new() -> Self {
        CompositMigrator {
            migrators: HashMap::new(),
        }
    }

    pub fn add_migrator(
        &mut self,
        from: V,
        to: V,
        migrate: impl Fn(T) -> Result<T, String> + 'static,
    ) {
        let edge = (from.clone(), to.clone());
        let migrator = Migrator {
            edge: edge.clone(),
            migrate: Box::new(migrate),
        };
        self.migrators.insert(edge, migrator);
    }

    pub fn migrate(&self, input: T, target: V) -> Result<T, String> {
        let input_version = input.version();

        let Some(found_path) = self.find_path(&input_version, &target) else {
            return Err(format!(
                "No path found from {input_version:?} to {target:?}"
            ));
        };

        let mut current = input;

        for i in 0..found_path.len() - 1 {
            let edge = (found_path[i].clone(), found_path[i + 1].clone());
            let migrator = self.migrators.get(&edge).unwrap();
            current = (migrator.migrate)(current)?;
            if current.version() != found_path[i + 1] {
                return Err(format!(
                    "Migration failed: expected version {:?}, got {:?}",
                    found_path[i + 1],
                    current.version()
                ));
            }
        }

        Ok(current)
    }

    fn find_path(&self, input_version: &V, target_version: &V) -> Option<Vec<V>> {
        let mut queue = vec![vec![input_version]];
        let mut visited = HashSet::new();

        while !queue.is_empty() {
            let path = queue.remove(0);
            let version = *path.last().unwrap();

            if target_version == version {
                return Some(path.iter().map(|item| item.clone().clone()).collect());
            }

            if !visited.contains(version) {
                visited.insert(version);

                // OPTIMIZE: lookup by hashing version tuple?
                for edge in self.migrators.keys() {
                    if edge.0 == *version {
                        let mut new_path = path.clone();
                        new_path.push(&edge.1);
                        queue.push(new_path);
                    }
                }
            }
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Config {
        V1(Vec<(u32, u32)>),
        V2(Vec<(u32, u32)>),
        V3(Vec<(u32, u32)>),
        V4(Vec<(u32, u32)>),
        V5(Vec<(u32, u32)>),
    }

    impl Versioned<u32> for Config {
        fn version(&self) -> u32 {
            match self {
                Config::V1(_) => 1,
                Config::V2(_) => 2,
                Config::V3(_) => 3,
                Config::V4(_) => 4,
                Config::V5(_) => 5,
            }
        }
    }

    fn migrator() -> CompositMigrator<Config, u32> {
        let mut migrator = CompositMigrator::new();

        migrator.add_migrator(1, 2, |config| {
            if let Config::V1(data) = config {
                let mut new_data = data.clone();
                new_data.push((1, 2));
                Ok(Config::V2(new_data))
            } else {
                Err("Invalid version".to_string())
            }
        });

        migrator.add_migrator(2, 3, |config| {
            if let Config::V2(data) = config {
                let mut new_data = data.clone();
                new_data.push((2, 3));
                Ok(Config::V3(new_data))
            } else {
                Err("Invalid version".to_string())
            }
        });

        migrator.add_migrator(3, 4, |config| {
            if let Config::V3(data) = config {
                let mut new_data: Vec<(u32, u32)> = data.clone();
                new_data.push((3, 4));
                Ok(Config::V4(new_data))
            } else {
                Err("Invalid version".to_string())
            }
        });

        migrator.add_migrator(1, 4, |config| {
            if let Config::V1(data) = config {
                let mut new_data: Vec<(u32, u32)> = data.clone();
                new_data.push((1, 4));
                Ok(Config::V4(new_data))
            } else {
                Err("Invalid version".to_string())
            }
        });

        migrator
    }

    fn test_pair(from: u32, to: u32) -> Result<Config, String> {
        let from_config = match from {
            1 => Config::V1(vec![]),
            2 => Config::V2(vec![]),
            3 => Config::V3(vec![]),
            4 => Config::V4(vec![]),
            5 => Config::V5(vec![]),
            _ => panic!("Invalid version"),
        };

        let migrator = migrator();
        migrator.migrate(from_config, to)
    }

    #[test]
    fn test_one_step() {
        let result = test_pair(1, 2);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Config::V2(vec![(1, 2)]));
    }

    #[test]
    fn test_multiple_steps() {
        let result = test_pair(1, 3);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Config::V3(vec![(1, 2), (2, 3)]));
    }

    #[test]
    fn test_reversed_path() {
        let result = test_pair(2, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_paths() {
        let result = test_pair(1, 4);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Config::V4(vec![(1, 4)]));
    }

    #[test]
    fn test_no_path() {
        let result = test_pair(1, 5);
        assert!(result.is_err());
    }
}
