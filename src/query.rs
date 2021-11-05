use fork::{fork, Fork};
use gtk::gdk::gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{
    Application, BoxBuilder, Entry, IconLookupFlags, IconTheme, Image, Label, ListBox, ListBoxRow,
    Orientation, ScrolledWindow, Viewport,
};
use nix::unistd::execvp;
use std::ffi::CString;
use std::process::{exit, Command};

pub fn init_query() -> Entry {
    let query_box = Entry::builder().name("findex-query").build();
    let desktop_entries = get_entries();

    query_box.style_context().add_class("findex-query");
    query_box.connect_changed({
        let de = desktop_entries.clone();
        move |qb| on_text_changed(qb, &de)
    });

    query_box
}

fn on_text_changed(qb: &Entry, apps: &Vec<AppInfo>) {
    let text = regex::escape(&qb.text().to_lowercase());
    if text.len() == 0 {
        let list_box = get_list_box(qb);
        clear_listbox(&list_box);
        return;
    }

    let regex = regex::Regex::new(&format!(r"^{}", text)).unwrap();

    let list_box = get_list_box(qb);

    clear_listbox(&list_box);

    for app in apps {
        if !regex.is_match(&app.name.to_lowercase()) {
            continue;
        }

        let icon = get_icon(&app.icon);

        let image = Image::builder().pixbuf(&icon).build();
        image.style_context().add_class("findex-result-icon");

        let name = Label::new(Some(&app.name));
        name.style_context().add_class("findex-result-app-name");

        let command = Label::new(Some(&app.exec));

        let container = BoxBuilder::new()
            .orientation(Orientation::Horizontal)
            .build();
        container.pack_start(&image, false, false, 0);
        container.pack_start(&name, false, false, 0);
        container.add(&command);

        let row = ListBoxRow::new();
        row.add(&container);
        row.style_context().add_class("findex-result-row");
        row.show_all();

        list_box.connect_row_activated(|_, lbr| {
            let container_w = &lbr.children()[0];
            let container = container_w.downcast_ref::<gtk::Box>().unwrap();
            let c_widget = &container.children()[2];
            let command = c_widget.downcast_ref::<Label>().unwrap();

            let splitted_cmd = shlex::split(&command.text().to_string());

            spawn_process(&splitted_cmd.unwrap());
        });
        list_box.add(&row);
    }
}

#[derive(Clone)]
struct AppInfo {
    name: String,
    exec: String,
    icon: String,
}
fn get_entries() -> Vec<AppInfo> {
    let apps_dir = std::fs::read_dir("/usr/share/applications/").unwrap();
    let mut apps = Vec::new();

    for app in apps_dir {
        let app = app.unwrap();
        let app_path = app.path();
        if app_path.is_dir() {
            continue;
        }
        if app_path.extension().unwrap().to_str().unwrap() != "desktop" {
            continue;
        }

        let desktop_entry = match freedesktop_entry_parser::parse_entry(&app_path) {
            Ok(entry) => entry,
            Err(e) => {
                eprintln!(
                    "Error occurred while parsing desktop entry: {}",
                    e.to_string()
                );
                continue;
            }
        };

        let section = desktop_entry.section("Desktop Entry");
        let name = section.attr("Name").unwrap();
        let icon = section.attr("Icon").unwrap_or("applications-other");
        let exec = match section.attr("Exec") {
            Some(e) => e,
            None => continue,
        };

        apps.push(AppInfo {
            name: name.to_string(),
            icon: icon.to_string(),
            exec: exec.to_string(),
        });
    }

    apps
}

fn get_icon(icon_name: &String) -> Pixbuf {
    let icon;
    let icon_theme = IconTheme::default().unwrap();

    if let Ok(i) = Pixbuf::from_file_at_size(&icon_name, 32, 32) {
        icon = i;
    } else if let Ok(i) = icon_theme.load_icon(
        icon_name,
        32,
        IconLookupFlags::FORCE_SIZE | IconLookupFlags::USE_BUILTIN,
    ) {
        icon = i.unwrap();
    } else {
        icon = icon_theme
            .load_icon(
                "applications-other",
                32,
                IconLookupFlags::FORCE_SIZE | IconLookupFlags::USE_BUILTIN,
            )
            .unwrap()
            .unwrap();
    }

    icon
}

fn clear_listbox(list_box: &ListBox) {
    for child in &list_box.children() {
        list_box.remove(child);
    }
}

fn get_list_box(qb: &Entry) -> ListBox {
    let par: gtk::Box = qb.parent().unwrap().downcast().unwrap();
    let child = &par.children()[1];
    let scw = child.downcast_ref::<ScrolledWindow>().unwrap();
    let scw_child = &scw.children()[0];
    let view_port = scw_child.downcast_ref::<Viewport>().unwrap();
    let v_child = &view_port.children()[0];

    v_child.downcast_ref::<ListBox>().unwrap().clone()
}

fn spawn_process(cmd: &Vec<String>) {
    let p_name = CString::new(cmd[0].as_bytes()).unwrap();
    execvp(
        &p_name,
        &cmd.iter()
            .map(|s| CString::new(s.as_bytes()).unwrap())
            .collect::<Vec<CString>>(),
    )
    .unwrap();
}