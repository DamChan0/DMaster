#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dmaster_core::{
    apply_profile, apply_profile_with_mapping, delete_profile, get_display_profile, load_profiles,
    save_profile, DisplayMapping, DisplayTopology,
};
use slint::{Model, ModelRc, SharedString, VecModel};
use std::rc::Rc;

slint::include_modules!();

fn format_timestamp(ts_str: &str) -> String {
    if let Ok(ts) = ts_str.parse::<u64>() {
        let days = ts / 86400;
        let years = days / 365;
        let remaining_days = days % 365;
        let months = remaining_days / 30;
        let day_of_month = (remaining_days % 30) + 1;
        format!("{:04}-{:02}-{:02}", 1970 + years, months + 1, day_of_month)
    } else {
        ts_str.to_string()
    }
}

fn topology_label(t: &DisplayTopology) -> &'static str {
    match t {
        DisplayTopology::Extend => "Extended",
        DisplayTopology::Clone => "Cloned",
        DisplayTopology::Internal => "Internal Only",
        DisplayTopology::External => "External Only",
        DisplayTopology::Unknown(_) => "Unknown",
    }
}

fn orientation_label(o: u32) -> &'static str {
    match o {
        1 => "90\u{00b0} Left",
        2 => "180\u{00b0} Inverted",
        3 => "90\u{00b0} Right",
        _ => "Normal",
    }
}

struct LoadedProfiles {
    entries: Vec<ProfileEntry>,
    profiles: Vec<dmaster_core::ProfileInfo>,
}

fn load_all_profiles() -> LoadedProfiles {
    match load_profiles() {
        Ok(profiles) => {
            let entries = profiles
                .iter()
                .map(|p| ProfileEntry {
                    name: SharedString::from(p.name.as_str()),
                    description: SharedString::from(
                        p.profile.description.as_deref().unwrap_or_default(),
                    ),
                    display_count: p.profile.displays.len() as i32,
                    created_at: format_timestamp(&p.profile.created_at).into(),
                    topology: SharedString::from(topology_label(&p.profile.topology)),
                })
                .collect();
            LoadedProfiles { entries, profiles }
        }
        Err(e) => {
            eprintln!("Failed to load profiles: {e}");
            LoadedProfiles {
                entries: vec![],
                profiles: vec![],
            }
        }
    }
}

fn displays_for_profile(info: &dmaster_core::ProfileInfo) -> Vec<DisplayEntry> {
    info.profile
        .displays
        .iter()
        .enumerate()
        .map(|(i, d)| DisplayEntry {
            name: SharedString::from(d.label.as_deref().unwrap_or(d.device_name.as_str())),
            resolution: format!("{}x{}", d.width, d.height).into(),
            position: format!("{}, {}", d.position_x, d.position_y).into(),
            orientation: SharedString::from(orientation_label(d.orientation)),
            is_primary: i == 0,
            is_enabled: d.enabled,
        })
        .collect()
}

fn build_profile_options(profile_info: &dmaster_core::ProfileInfo) -> Vec<SharedString> {
    let mut options: Vec<SharedString> = vec![SharedString::from("(skip)")];
    for (i, d) in profile_info.profile.displays.iter().enumerate() {
        let label = d.label.as_deref().unwrap_or(d.device_name.as_str());
        options.push(SharedString::from(format!(
            "[{}] {} {}x{}",
            i, label, d.width, d.height
        )));
    }
    options
}

fn main() {
    let app = App::new().expect("Failed to create Slint window");

    let loaded = load_all_profiles();
    let model: Rc<VecModel<ProfileEntry>> = Rc::new(VecModel::from(loaded.entries));
    let profile_data: Rc<std::cell::RefCell<Vec<dmaster_core::ProfileInfo>>> =
        Rc::new(std::cell::RefCell::new(loaded.profiles));
    let display_model: Rc<VecModel<DisplayEntry>> = Rc::new(VecModel::default());
    let mapping_model: Rc<VecModel<MappingRow>> = Rc::new(VecModel::default());
    let mapping_selections: Rc<std::cell::RefCell<Vec<i32>>> =
        Rc::new(std::cell::RefCell::new(vec![]));
    let mapping_enabled: Rc<std::cell::RefCell<Vec<bool>>> =
        Rc::new(std::cell::RefCell::new(vec![]));
    let pending_mapping_profile: Rc<std::cell::RefCell<Option<String>>> =
        Rc::new(std::cell::RefCell::new(None));

    app.set_profiles(ModelRc::from(model.clone()));
    app.set_selected_displays(ModelRc::from(display_model.clone()));
    app.set_mapping_rows(ModelRc::from(mapping_model.clone()));
    app.set_status_message("Ready".into());

    let display_model_clone = display_model.clone();
    let profile_data_clone = profile_data.clone();
    app.on_select_profile(move |index: i32| {
        let data = profile_data_clone.borrow();
        if let Some(info) = data.get(index as usize) {
            display_model_clone.set_vec(displays_for_profile(info));
        } else {
            display_model_clone.set_vec(vec![]);
        }
    });

    let display_model_clone = display_model.clone();
    app.on_display_toggled(move |di: i32, enabled: bool| {
        if let Some(mut entry) = display_model_clone.row_data(di as usize) {
            entry.is_enabled = enabled;
            display_model_clone.set_row_data(di as usize, entry);
        }
    });

    let app_weak = app.as_weak();
    let model_clone = model.clone();
    let profile_data_clone = profile_data.clone();
    let display_model_clone = display_model.clone();
    app.on_refresh_profiles(move || {
        let loaded = load_all_profiles();
        model_clone.set_vec(loaded.entries);
        *profile_data_clone.borrow_mut() = loaded.profiles;
        display_model_clone.set_vec(vec![]);
        if let Some(app) = app_weak.upgrade() {
            app.set_selected_index(-1);
            app.set_status_message("Profiles refreshed".into());
            app.set_status_is_error(false);
        }
    });

    let app_weak = app.as_weak();
    let model_clone = model.clone();
    let profile_data_clone = profile_data.clone();
    let display_model_clone = display_model.clone();
    app.on_save_profile(move |name: SharedString, desc: SharedString| {
        let desc_opt = if desc.is_empty() {
            None
        } else {
            Some(desc.to_string())
        };

        let result = get_display_profile().and_then(|profile| {
            save_profile(name.as_str(), desc_opt, &profile).map(|path| path.display().to_string())
        });

        if let Some(app) = app_weak.upgrade() {
            match result {
                Ok(path) => {
                    app.set_status_message(format!("Saved to {path}").into());
                    app.set_status_is_error(false);
                    let loaded = load_all_profiles();
                    model_clone.set_vec(loaded.entries);
                    *profile_data_clone.borrow_mut() = loaded.profiles;
                    display_model_clone.set_vec(vec![]);
                    app.set_selected_index(-1);
                }
                Err(e) => {
                    app.set_status_message(format!("Save failed: {e}").into());
                    app.set_status_is_error(true);
                }
            }
        }
    });

    let app_weak = app.as_weak();
    app.on_apply_profile(move |name: SharedString| {
        let result = dmaster_core::load_profile_by_name(name.as_str())
            .and_then(|info| apply_profile(&info.profile.to_display_profile()));

        if let Some(app) = app_weak.upgrade() {
            match result {
                Ok(()) => {
                    app.set_status_message(format!("Applied '{}'", name.as_str()).into());
                    app.set_status_is_error(false);
                }
                Err(e) => {
                    app.set_status_message(format!("Apply failed: {e}").into());
                    app.set_status_is_error(true);
                }
            }
        }
    });

    let app_weak = app.as_weak();
    let display_model_clone = display_model.clone();
    app.on_request_apply(move |name: SharedString| {
        let profile_info = match dmaster_core::load_profile_by_name(name.as_str()) {
            Ok(p) => p,
            Err(e) => {
                if let Some(app) = app_weak.upgrade() {
                    app.set_status_message(format!("Load failed: {e}").into());
                    app.set_status_is_error(true);
                }
                return;
            }
        };

        let mut profile_to_apply = profile_info.profile.to_display_profile();
        for (i, display) in profile_to_apply.displays.iter_mut().enumerate() {
            if let Some(entry) = display_model_clone.row_data(i) {
                display.enabled = entry.is_enabled;
            }
        }

        let result = apply_profile(&profile_to_apply);
        if let Some(app) = app_weak.upgrade() {
            match result {
                Ok(()) => {
                    app.set_status_message(format!("Applied '{}'", name.as_str()).into());
                    app.set_status_is_error(false);
                }
                Err(e) => {
                    app.set_status_message(format!("Apply failed: {e}").into());
                    app.set_status_is_error(true);
                }
            }
        }
    });

    let mapping_selections_clone = mapping_selections.clone();
    app.on_mapping_selection_changed(move |row_idx: i32, selection_idx: i32| {
        let mut selections = mapping_selections_clone.borrow_mut();
        if let Some(slot) = selections.get_mut(row_idx as usize) {
            *slot = selection_idx;
        }
    });

    let mapping_enabled_clone = mapping_enabled.clone();
    app.on_mapping_output_toggled(move |row_idx: i32, enabled: bool| {
        let mut enabled_vec = mapping_enabled_clone.borrow_mut();
        if let Some(slot) = enabled_vec.get_mut(row_idx as usize) {
            *slot = enabled;
        }
    });

    let app_weak = app.as_weak();
    let mapping_selections_clone = mapping_selections.clone();
    let mapping_enabled_clone = mapping_enabled.clone();
    let pending_mapping_profile_clone = pending_mapping_profile.clone();
    app.on_mapping_confirmed(move || {
        let profile_name = match pending_mapping_profile_clone.borrow().clone() {
            Some(name) => name,
            None => return,
        };

        let profile_info = match dmaster_core::load_profile_by_name(&profile_name) {
            Ok(p) => p,
            Err(e) => {
                if let Some(app) = app_weak.upgrade() {
                    app.set_status_message(format!("Load failed: {e}").into());
                    app.set_status_is_error(true);
                }
                return;
            }
        };

        let current = match get_display_profile() {
            Ok(p) => p,
            Err(e) => {
                if let Some(app) = app_weak.upgrade() {
                    app.set_status_message(format!("Query failed: {e}").into());
                    app.set_status_is_error(true);
                }
                return;
            }
        };

        let selections = mapping_selections_clone.borrow();
        let enabled_vec = mapping_enabled_clone.borrow();
        let mut mappings = Vec::new();
        let mut disabled_display_names = Vec::new();

        for (i, &sel) in selections.iter().enumerate() {
            let is_enabled = enabled_vec.get(i).copied().unwrap_or(true);
            if !is_enabled {
                if let Some(display) = current.displays.get(i) {
                    disabled_display_names.push(display.device_name.clone());
                }
                continue;
            }
            if sel == 0 {
                continue;
            }
            let profile_display_index = (sel - 1) as usize;
            if let Some(display) = current.displays.get(i) {
                mappings.push(DisplayMapping {
                    current_display_name: display.device_name.clone(),
                    profile_display_index,
                });
            }
        }

        if mappings.is_empty() && disabled_display_names.is_empty() {
            if let Some(app) = app_weak.upgrade() {
                app.set_status_message("No mappings or changes selected".into());
                app.set_status_is_error(true);
            }
            return;
        }

        let mut result =
            apply_profile_with_mapping(&profile_info.profile.to_display_profile(), &mappings);

        if result.is_ok() && !disabled_display_names.is_empty() {
            let disabled_profile = dmaster_core::DisplayProfile {
                topology: profile_info.profile.topology.clone(),
                displays: disabled_display_names
                    .iter()
                    .map(|name| dmaster_core::DisplayConfig {
                        label: None,
                        device_name: name.clone(),
                        device_id: String::new(),
                        device_key: String::new(),
                        width: 0,
                        height: 0,
                        position_x: 0,
                        position_y: 0,
                        orientation: 0,
                        enabled: false,
                    })
                    .collect(),
            };
            result = apply_profile(&disabled_profile);
        }

        if let Some(app) = app_weak.upgrade() {
            match result {
                Ok(()) => {
                    app.set_status_message(
                        format!("Applied '{}' with mapping", profile_name).into(),
                    );
                    app.set_status_is_error(false);
                }
                Err(e) => {
                    app.set_status_message(format!("Apply failed: {e}").into());
                    app.set_status_is_error(true);
                }
            }
        }
    });

    let app_weak = app.as_weak();
    let model_clone = model.clone();
    let profile_data_clone = profile_data.clone();
    let display_model_clone = display_model.clone();
    app.on_delete_profile(move |name: SharedString| {
        let result = delete_profile(name.as_str());

        if let Some(app) = app_weak.upgrade() {
            match result {
                Ok(()) => {
                    app.set_status_message(format!("Deleted '{}'", name.as_str()).into());
                    app.set_status_is_error(false);
                    app.set_selected_index(-1);
                    let loaded = load_all_profiles();
                    model_clone.set_vec(loaded.entries);
                    *profile_data_clone.borrow_mut() = loaded.profiles;
                    display_model_clone.set_vec(vec![]);
                }
                Err(e) => {
                    app.set_status_message(format!("Delete failed: {e}").into());
                    app.set_status_is_error(true);
                }
            }
        }
    });

    let app_weak = app.as_weak();
    app.window().on_close_requested(move || {
        if let Some(app) = app_weak.upgrade() {
            app.set_show_quit_dialog(true);
        }
        slint::CloseRequestResponse::KeepWindowShown
    });

    let app_weak = app.as_weak();
    app.on_confirm_quit(move || {
        if let Some(app) = app_weak.upgrade() {
            app.hide().ok();
        }
        slint::quit_event_loop().ok();
    });

    app.run().expect("Slint event loop failed");
}
