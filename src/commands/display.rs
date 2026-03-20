use crate::cli::DisplayCommands;
use log::error;
use std::collections::HashMap;

use wayland_client::{Connection, Dispatch, Proxy, QueueHandle, protocol::wl_registry};
use wayland_protocols_wlr::output_management::v1::client::{
    zwlr_output_configuration_head_v1::ZwlrOutputConfigurationHeadV1,
    zwlr_output_configuration_v1::{self, ZwlrOutputConfigurationV1},
    zwlr_output_head_v1::{self, ZwlrOutputHeadV1},
    zwlr_output_manager_v1::{self, ZwlrOutputManagerV1},
    zwlr_output_mode_v1::{self, ZwlrOutputModeV1},
};

pub fn execute(command: &DisplayCommands) {
    match command {
        DisplayCommands::Rates => {
            show_refresh_rates();
        }
        DisplayCommands::SetRate { rate } => {
            set_refresh_rate(*rate);
        }
    }
}

// State Structs
#[derive(Debug, Default, Clone)]
pub struct WlMode {
    pub width: i32,
    pub height: i32,
    pub refresh: i32,
    pub preferred: bool,
}

#[derive(Debug, Clone)]
pub struct WlHead {
    pub name: String,
    pub description: String,
    pub make: String,
    pub model: String,
    pub enabled: bool,
    pub current_mode: Option<ZwlrOutputModeV1>,
    pub x: i32,
    pub y: i32,
    pub scale: f64,
    pub transform: wayland_client::protocol::wl_output::Transform,
}

impl Default for WlHead {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            make: String::new(),
            model: String::new(),
            enabled: false,
            current_mode: None,
            x: 0,
            y: 0,
            scale: 1.0,
            transform: wayland_client::protocol::wl_output::Transform::Normal,
        }
    }
}

pub struct AppState {
    pub manager: Option<ZwlrOutputManagerV1>,
    pub heads: HashMap<ZwlrOutputHeadV1, WlHead>,
    pub modes: HashMap<ZwlrOutputModeV1, (ZwlrOutputHeadV1, WlMode)>,
    pub done: bool,
    pub config_success: Option<bool>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for AppState {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        else {
            return;
        };
        if interface == "zwlr_output_manager_v1" {
            let manager = registry.bind::<ZwlrOutputManagerV1, _, _>(name, version, qh, ());
            state.manager = Some(manager);
        }
    }
}

impl Dispatch<ZwlrOutputManagerV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _: &ZwlrOutputManagerV1,
        event: zwlr_output_manager_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_manager_v1::Event::Head { head } => {
                state.heads.insert(head, WlHead::default());
            }
            zwlr_output_manager_v1::Event::Done { .. } => {
                state.done = true;
            }
            _ => {}
        }
    }

    fn event_created_child(
        opcode: u16,
        qh: &QueueHandle<Self>,
    ) -> std::sync::Arc<dyn wayland_client::backend::ObjectData> {
        match opcode {
            0 => qh.make_data::<ZwlrOutputHeadV1, ()>(()),
            _ => panic!(
                "Missing event_created_child specialization for opcode {}",
                opcode
            ),
        }
    }
}

impl Dispatch<ZwlrOutputHeadV1, ()> for AppState {
    fn event(
        state: &mut Self,
        head: &ZwlrOutputHeadV1,
        event: zwlr_output_head_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let Some(h) = state.heads.get_mut(head) {
            match event {
                zwlr_output_head_v1::Event::Name { name } => h.name = name,
                zwlr_output_head_v1::Event::Description { description } => {
                    h.description = description
                }
                zwlr_output_head_v1::Event::Make { make } => h.make = make,
                zwlr_output_head_v1::Event::Model { model } => h.model = model,
                // Mode event gives us a new mode object!
                zwlr_output_head_v1::Event::Mode { mode } => {
                    state.modes.insert(mode, (head.clone(), WlMode::default()));
                }
                zwlr_output_head_v1::Event::Enabled { enabled } => h.enabled = enabled != 0,
                zwlr_output_head_v1::Event::CurrentMode { mode } => h.current_mode = Some(mode),
                zwlr_output_head_v1::Event::Position { x, y } => {
                    h.x = x;
                    h.y = y;
                }
                zwlr_output_head_v1::Event::Scale { scale } => h.scale = scale,
                zwlr_output_head_v1::Event::Transform {
                    transform: wayland_client::WEnum::Value(t),
                } => {
                    h.transform = t;
                }
                _ => {}
            }
        }
    }

    fn event_created_child(
        opcode: u16,
        qh: &QueueHandle<Self>,
    ) -> std::sync::Arc<dyn wayland_client::backend::ObjectData> {
        match opcode {
            3 => qh.make_data::<ZwlrOutputModeV1, ()>(()),
            _ => panic!(
                "Missing event_created_child specialization for opcode {} on head",
                opcode
            ),
        }
    }
}

impl Dispatch<ZwlrOutputModeV1, ()> for AppState {
    fn event(
        state: &mut Self,
        mode: &ZwlrOutputModeV1,
        event: zwlr_output_mode_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let Some((head_handle, mut wl_mode)) = state.modes.remove(mode) {
            match event {
                zwlr_output_mode_v1::Event::Size { width, height } => {
                    wl_mode.width = width;
                    wl_mode.height = height;
                }
                zwlr_output_mode_v1::Event::Refresh { refresh } => {
                    wl_mode.refresh = refresh;
                }
                zwlr_output_mode_v1::Event::Preferred => {
                    wl_mode.preferred = true;
                }
                _ => {}
            }
            state
                .modes
                .insert(mode.clone(), (head_handle.clone(), wl_mode));
        }
    }
}

impl Dispatch<ZwlrOutputConfigurationV1, ()> for AppState {
    fn event(
        state: &mut Self,
        _: &ZwlrOutputConfigurationV1,
        event: zwlr_output_configuration_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            zwlr_output_configuration_v1::Event::Succeeded => state.config_success = Some(true),
            zwlr_output_configuration_v1::Event::Failed => state.config_success = Some(false),
            zwlr_output_configuration_v1::Event::Cancelled => state.config_success = Some(false),
            _ => {}
        }
    }
}

impl Dispatch<ZwlrOutputConfigurationHeadV1, ()> for AppState {
    fn event(
        _: &mut Self,
        _: &ZwlrOutputConfigurationHeadV1,
        _: wayland_protocols_wlr::output_management::v1::client::zwlr_output_configuration_head_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

fn fetch_state() -> Option<(Connection, AppState, wayland_client::EventQueue<AppState>)> {
    let conn = Connection::connect_to_env().ok()?;
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let display = conn.display();

    let mut state = AppState {
        manager: None,
        heads: HashMap::new(),
        modes: HashMap::new(),
        done: false,
        config_success: None,
    };

    display.get_registry(&qh, ());
    event_queue.roundtrip(&mut state).ok()?;

    if state.manager.is_none() {
        error!("Your compositor does not support zwlr_output_manager_v1.");
        return None;
    }

    // Two roundtrips to fetch heads then modes
    event_queue.roundtrip(&mut state).ok()?;
    event_queue.roundtrip(&mut state).ok()?;

    Some((conn, state, event_queue))
}

pub fn get_active_display_info() -> Vec<String> {
    let mut infos = Vec::new();
    if let Some((_, state, _)) = fetch_state() {
        for (_, head) in state.heads.iter() {
            if !head.enabled {
                continue;
            }
            let Some(current_mode_handle) = &head.current_mode else {
                continue;
            };
            let Some((_, mode)) = state.modes.get(current_mode_handle) else {
                continue;
            };

            let hz = (mode.refresh as f64) / 1000.0;
            infos.push(format!(
                "{} ({}): {}x{} px, {:.3} Hz",
                head.name, head.make, mode.width, mode.height, hz
            ));
        }
    }
    infos
}

fn show_refresh_rates() {
    if let Some((_, state, _)) = fetch_state() {
        for (head_handle, head) in state.heads.iter() {
            println!("{} \"{}\"", head.name, head.description);
            println!("  Make: {}", head.make);
            println!("  Model: {}", head.model);
            println!("  Enabled: {}", if head.enabled { "yes" } else { "no" });
            println!("  Modes:");

            // Gather modes for this head
            let mut head_modes: Vec<(&ZwlrOutputModeV1, &WlMode)> = state
                .modes
                .iter()
                .filter(|(_, (mh, _))| mh == head_handle)
                .map(|(mode_handle, (_, mode))| (mode_handle, mode))
                .collect();

            // Sort by resolution descending, then refresh rate descending
            head_modes.sort_by(|a, b| {
                b.1.width
                    .cmp(&a.1.width)
                    .then(b.1.height.cmp(&a.1.height))
                    .then(b.1.refresh.cmp(&a.1.refresh))
            });

            // Deduplicate modes based on properties
            let mut seen = std::collections::HashSet::new();
            for (mode_handle, mode) in head_modes {
                if mode.width != 0 && mode.height != 0 && mode.refresh != 0 {
                    let key = format!("{}x{}@{}", mode.width, mode.height, mode.refresh);

                    let is_current = head.current_mode.as_ref() == Some(mode_handle);
                    let is_preferred = mode.preferred;

                    // Always show current/preferred, otherwise deduplicate identical ones
                    if seen.insert(key) || is_current || is_preferred {
                        let hz = (mode.refresh as f64) / 1000.0;
                        let mut markers = Vec::new();

                        if is_preferred {
                            markers.push("preferred");
                        }
                        if is_current {
                            markers.push("current");
                        }

                        let marker_str = if markers.is_empty() {
                            String::new()
                        } else {
                            format!(" ({})", markers.join(", "))
                        };
                        println!(
                            "    {}x{} px, {:.6} Hz{}",
                            mode.width, mode.height, hz, marker_str
                        );
                    }
                }
            }
        }
    } else {
        error!("Native Wayland configuration failed. Are you on a wlroots compositor?");
    }
}

fn set_refresh_rate(rate: f32) {
    let Some((_conn, mut state, mut eq)) = fetch_state() else {
        return;
    };
    let Some(manager) = state.manager.clone() else {
        return;
    };
    let qh = eq.handle();
    let config = manager.create_configuration(manager.version(), &qh, ());

    if let Some((head_handle, head_info)) = state.heads.iter().find(|(_, h)| h.enabled) {
        let target_mhz = (rate * 1000.0) as i32;

        for (mode_handle, (mh, mode_info)) in state.modes.iter() {
            if mh == head_handle {
                // fuzzy match the mHz
                if (mode_info.refresh - target_mhz).abs() < 5000 {
                    let head_config = config.enable_head(head_handle, &qh, ());
                    head_config.set_mode(mode_handle);
                    head_config.set_position(head_info.x, head_info.y);
                    head_config.set_scale(head_info.scale);
                    head_config.set_transform(head_info.transform);

                    config.apply();

                    state.config_success = None;
                    while state.config_success.is_none() {
                        if eq.blocking_dispatch(&mut state).is_err() {
                            break;
                        }
                    }

                    if state.config_success == Some(true) {
                        println!("[SUCCESS] Applied Wayland output configuration!");
                    } else {
                        error!("[FAILED] Compositor rejected the output configuration request.");
                    }
                    return;
                }
            }
        }
    }
}
