use dmaster_core::{
    apply_profile, delete_profile, get_display_profile, load_profiles, save_profile,
    DisplayTopology,
};
use slint::{ModelRc, SharedString, VecModel};
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
        })
        .collect()
}

fn main() {
    let app = App::new().expect("Failed to create Slint window");

    let loaded = load_all_profiles();
    let model: Rc<VecModel<ProfileEntry>> = Rc::new(VecModel::from(loaded.entries));
    let profile_data: Rc<std::cell::RefCell<Vec<dmaster_core::ProfileInfo>>> =
        Rc::new(std::cell::RefCell::new(loaded.profiles));
    let display_model: Rc<VecModel<DisplayEntry>> = Rc::new(VecModel::default());

    app.set_profiles(ModelRc::from(model.clone()));
    app.set_selected_displays(ModelRc::from(display_model.clone()));
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

    app.run().expect("Slint event loop failed");
}
