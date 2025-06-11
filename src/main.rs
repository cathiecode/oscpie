mod component;
mod components;
mod config;
mod debug;
mod openvr;
mod prelude;
mod resource;
mod sprite;
mod story;
mod types;
mod utils;
mod versioned;
mod vulkan;

use std::f32::consts::PI;

use crate::{
    debug::{debug_window, rt_debug},
    prelude::*,
};
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
}

trait App {
    fn on_update(&mut self, input: AppInput) -> Result<()> {
        Ok(())
    }
    fn on_render(&mut self, _: &mut Pixmap) -> Result<()> {
        Ok(())
    }
}

struct AppImpl {
    fps: Fps,
    interval_timer_update: IntervalTimer,
    interval_timer_render: IntervalTimer,
    should_render: bool,
    pie_menu: pie_menu::PieMenuComponent,
}

impl AppImpl {
    fn new(configuration: Config) -> Result<Self> {
        Ok(Self {
            fps: Fps::new(60),
            interval_timer_update: IntervalTimer::new(1000.0),
            interval_timer_render: IntervalTimer::new(1000.0),
            should_render: true,
            pie_menu: Self::create_pie_menu(&configuration.root, &configuration),
        })
    }

    fn create_pie_menu(
        menu_id: &config::types::MenuId,
        configuration: &Config,
    ) -> pie_menu::PieMenuComponent {
        let center_x = 256.0;
        let center_y = 256.0;
        let radius = 256.0 * 0.9;

        let menu: Menu = configuration.menus.get(menu_id).unwrap().clone().into(); // OPTIMIZE: do not clone

        pie_menu::PieMenuComponent::new(center_x, center_y, radius, menu)
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
        } = input;

        self.pie_menu.update(&pie_menu::Props {
            pie_menu_input: PieMenuInput {
                angle,
                magnitude,
                click,
            },
        });

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

        pixmap.fill(tiny_skia::Color::from_rgba(0.0, 0.0, 0.0, 0.0).unwrap());
        if self.should_render {
            self.should_render = false;
        } else {
            return Ok(());
        }

        self.pie_menu.render(pixmap);

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

    let mut app = AppImpl::new(config)?;

    let openvr = openvr::Handle::<openvr::OpenVr>::new(openvr::EVRApplicationType::Overlay)?;
    let overlay_interface = openvr.overlay()?;
    let compositor = openvr.compositor()?;

    let action_manifest_path = resolve_path("config", "action_manifests.json");

    let mut input = openvr.input(Some(action_manifest_path))?;

    input.activate_actions_main();
    let overlay = overlay_interface.create("oscpie_overlay", "OSCPie Overlay")?;
    overlay.show()?;
    let mut pixmap = Pixmap::new(512, 512).unwrap();
    let mut uploader = vulkan::ImageUploader::new(&pixmap, compositor.clone())?;

    let mut interval_timer = IntervalTimer::new(1000.0);

    let demo = false;

    std::thread::spawn(move || debug_window());

    loop {
        let timing = TimingCheck::new();

        let input = if demo {
            let time_as_seconds = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs_f32();

            let angle = ((time_as_seconds * PI * 2.0 * 0.1) % (PI * 2.0));
            let magnitude = f32::midpoint((time_as_seconds * PI * 2.0 * 1.0).cos(), 1.0);

            AppInput {
                angle,
                magnitude,
                click: 0.0,
            }
        } else {
            input.update()?;
            let click_input = input.get_actions_main_in_ClickLeft()?;
            let select_input = input.get_actions_main_in_SelectLeft()?;
            let pose = input
                .get_actions_main_in_PoseLeft(openvr::TrackingUniverseOrigin::RawAndUncalibrated)?;

            if pose.active {
                overlay.set_overlay_transform_absolute(
                    openvr::TrackingUniverseOrigin::RawAndUncalibrated,
                    pose.pose.unwrap(),
                )?;
            }

            rt_debug(|| (format!("20_click"), format!("ClickLeft: {click_input:?}, SelectLeft: {select_input:?}")));

            rt_debug(|| (format!("30_pose"), format!("PoseLeft: {:?}, Active: {}", pose.pose, pose.active)));

            AppInput {
                angle: (-select_input.value.y).atan2(select_input.value.x).rem_euclid(PI * 2.0),
                magnitude: select_input.value.length(),
                click: if click_input.state { 1.0 } else { 0.0 },
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
            rt_debug(|| (format!("10_FPS"), format!("whole process: {time_elapsed_ns}ns")));
        }

        overlay.wait_frame_sync(100)?;
    }
}

fn main() {
    env_logger::init();
    app().unwrap();
}
