mod action_behaviours;
mod component;
mod components;
mod config;
mod debug;
mod menu;
mod openvr;
mod prelude;
mod resource;
mod sprite;
mod story;
mod utils;
mod versioned;
mod vulkan;

use std::{cell::RefCell, collections::HashMap, f32::consts::PI, rc::Rc};

use crate::{debug::rt_debug, prelude::*};
use anyhow::Result;
use components::pie_menu;
use config::Config;
use resource::SPRITE_SHEET;
use sprite::SpriteSheet;
use tiny_skia::Pixmap;

struct AppInput {
    angle: f32,
    magnitude: f32,
    click: f32,
    open_menu: bool,
}

trait App {
    fn on_update(&mut self, _input: AppInput) -> Result<()>;
    fn on_render(&mut self, _: &mut Pixmap) -> Result<()>;
}

struct AppImpl {
    fps: Fps,
    interval_timer_update: IntervalTimer,
    interval_timer_render: IntervalTimer,
    should_render: bool,
    current_pie_menu_component: pie_menu::PieMenuComponent,
    menu_map: HashMap<MenuId, Menu>,
    received_events: Rc<RefCell<Vec<AppEvent>>>,
    menu_stack: Vec<MenuId>,
    is_open: bool,
    open_menu_state_machine: ClickStateMachine,
}

impl AppImpl {
    fn new(configuration: &Config) -> AppImpl {
        let received_events = Rc::new(RefCell::new(Vec::new()));

        let mut menu_map = HashMap::new();

        for (id, menu) in &configuration.menus {
            let menu: Menu = Menu::from_config(menu, &received_events);
            menu_map.insert(MenuId::from_config(id), menu);
        }

        Self {
            fps: Fps::new(60),
            interval_timer_update: IntervalTimer::new(1000.0),
            interval_timer_render: IntervalTimer::new(1000.0),
            should_render: true,
            current_pie_menu_component: Self::create_pie_menu(
                menu_map
                    .get(&MenuId::from_config(&configuration.root))
                    .unwrap(),
            ),
            menu_map,
            received_events,
            menu_stack: vec![MenuId::from_config(&configuration.root)],
            is_open: false,
            open_menu_state_machine: ClickStateMachine::new(),
        }
    }

    fn create_pie_menu(menu: &Menu) -> pie_menu::PieMenuComponent {
        let center_x = 256.0;
        let center_y = 256.0;
        let radius = 256.0 * 0.9;

        pie_menu::PieMenuComponent::new(center_x, center_y, radius, menu)
    }

    fn replace_pie_menu(&mut self) {
        let Some(menu_id) = self.menu_stack.last().cloned() else {
            log::error!("No menu ID found in the stack");
            return;
        };

        if let Some(menu) = self.menu_map.get(&menu_id) {
            let mut menu = menu.clone();

            if self.menu_stack.len() > 1 {
                let back_action = self.app_action(AppEvent::PopStack);

                let back_item = MenuItem::new(back_action, Some("back".to_string()));
                menu.items.insert(0, back_item);
            }

            self.current_pie_menu_component = Self::create_pie_menu(&menu);
        } else {
            log::error!("Menu with ID {menu_id:?} not found");
        }
    }

    fn app_action(&mut self, app_event: AppEvent) -> MenuItemAction {
        MenuItemAction::OneShotButton(Rc::new(RefCell::new(AppEventMenuActionBehaviour::new(
            self.received_events.clone(),
            app_event,
        ))))
    }
}

impl App for AppImpl {
    fn on_update(&mut self, input: AppInput) -> Result<()> {
        let timing_check = TimingCheck::new();
        self.should_render = true;

        let AppInput {
            angle,
            magnitude,
            click,
            open_menu,
        } = input;

        let open_menu_state_machine_event = self.open_menu_state_machine.update(open_menu);

        if let Some(ClickStateMachineEvent::Click) = open_menu_state_machine_event {
            self.is_open = !self.is_open;
        }

        // Cull if the menu is not open
        if !self.is_open {
            return Ok(());
        }

        let mut should_replace_menu = false;

        {
            let mut recived_events = self.received_events.borrow_mut();

            for event in recived_events.iter() {
                match event {
                    AppEvent::PopStack => {
                        if self.menu_stack.len() > 1 {
                            self.menu_stack.pop();
                            should_replace_menu = true;
                        } else {
                            log::warn!("Attempted to pop the root menu, ignoring.");
                        }
                    }
                    AppEvent::PushStack(to) => {
                        self.menu_stack.push(to.clone());
                        should_replace_menu = true;
                    }
                }
            }

            if !recived_events.is_empty() {
                recived_events.clear();
            }
        }

        if should_replace_menu {
            self.replace_pie_menu();
        }

        self.current_pie_menu_component
            .update(&pie_menu::Props::new(PieMenuInput {
                angle,
                magnitude,
                click,
            }));

        self.fps.update();

        let time_elapsed_ns = timing_check.get_time_ns();

        if self.interval_timer_update.update() {
            log::info!("update: {time_elapsed_ns}ns");
            log::info!("fps: {}", self.fps.get_fps());
        }

        Ok(())
    }

    fn on_render(&mut self, pixmap: &mut Pixmap) -> Result<()> {
        let timing_check = TimingCheck::new();

        if !self.is_open {
            return Ok(());
        }

        pixmap.fill(tiny_skia::Color::from_rgba(0.0, 0.0, 0.0, 0.0).unwrap());
        if self.should_render {
            self.should_render = false;
        } else {
            return Ok(());
        }

        self.current_pie_menu_component.render(pixmap);

        if self.interval_timer_render.update() {
            log::info!("render: {}ns", timing_check.get_time_ns());
        }

        Ok(())
    }
}

fn app() -> Result<()> {
    let config = config::load("config/config.json")?;

    SPRITE_SHEET
        .set(SpriteSheet::load(resolve_path("config/config.json", &config.sprite_sheet)).unwrap())
        .unwrap();

    let mut app = AppImpl::new(&config);

    let openvr = openvr::Handle::<openvr::OpenVr>::new(openvr::EVRApplicationType::Overlay)?;
    let overlay_interface = openvr.overlay()?;
    let compositor = openvr.compositor()?;

    let action_manifest_path = resolve_path("config", "action_manifests.json");

    let mut input = openvr.input(Some(action_manifest_path))?;

    input.activate_actions_main();
    let overlay = overlay_interface.create("oscpie_overlay", "OSCPie Overlay")?;
    overlay.show()?;
    let mut pixmap = Pixmap::new(512, 512).unwrap();
    let mut uploader = vulkan::ImageUploader::new(&pixmap, &compositor)?;

    let mut interval_timer = IntervalTimer::new(1000.0);

    let demo = false;

    // std::thread::spawn(move || debug_window());

    loop {
        let timing = TimingCheck::new();

        let input = if demo {
            let time_as_seconds = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f32();

            let angle = (time_as_seconds * PI * 2.0 * 0.1) % (PI * 2.0);
            let magnitude = f32::midpoint((time_as_seconds * PI * 2.0 * 1.0).cos(), 1.0);

            AppInput {
                angle,
                magnitude,
                click: 0.0,
                open_menu: false,
            }
        } else {
            input.update()?;
            let click_input = input.get_actions_main_in_ClickLeft()?;
            let select_input = input.get_actions_main_in_SelectLeft()?;
            let open_menu_input = input.get_actions_main_in_OpenLeft()?;
            let pose = input
                .get_actions_main_in_PoseLeft(openvr::TrackingUniverseOrigin::RawAndUncalibrated)?;

            if pose.active {
                overlay.set_overlay_transform_absolute(
                    openvr::TrackingUniverseOrigin::RawAndUncalibrated,
                    pose.pose.unwrap(),
                )?;
            }

            rt_debug(|| {
                (
                    "20_click".to_string(),
                    format!("ClickLeft: {click_input:?}, SelectLeft: {select_input:?}"),
                )
            });

            rt_debug(|| {
                (
                    "30_pose".to_string(),
                    format!("PoseLeft: {:?}, Active: {}", pose.pose, pose.active),
                )
            });

            AppInput {
                angle: (-select_input.value.y)
                    .atan2(select_input.value.x)
                    .rem_euclid(PI * 2.0),
                magnitude: select_input.value.length(),
                click: if click_input.state { 1.0 } else { 0.0 },
                open_menu: open_menu_input.state,
            }
        };

        app.on_update(input)?;
        app.on_render(&mut pixmap)?;

        let image = uploader.upload(&pixmap);

        let texture_handle = openvr::TextureHandle::Vulkan(image.as_ref(), uploader.queue());

        let mut texture = openvr::Texture {
            handle: texture_handle,
            texture_type: openvr::TextureType::Vulkan,
            color_space: openvr::ColorSpace::Auto,
        };

        overlay.set_overlay_texture(&mut texture)?;

        let time_elapsed_ns = timing.get_time_ns();
        if interval_timer.update() {
            rt_debug(|| {
                (
                    "10_FPS".to_string(),
                    format!("whole process: {time_elapsed_ns}ns"),
                )
            });
        }

        if app.is_open {
            overlay.show()?;
        } else {
            overlay.hide()?;
        }

        overlay.wait_frame_sync(100)?;
    }
}

fn main() {
    env_logger::init();
    app().unwrap();
}
