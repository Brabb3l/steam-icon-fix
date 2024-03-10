use std::path::Path;
use clap::Parser;
use colored::*;
use ini::{Ini, ParseOption};

const STEAM_CDN: &str = "https://steamcdn-a.akamaihd.net/steamcommunity/public/images/apps";

macro_rules! error {
    ($($arg:tt)*) => {
        {
            println!("{}: {}", "error".red().bold(), format!($($arg)*));
            std::process::exit(1)
        }
    }
}

macro_rules! warn {
    ($($arg:tt)*) => {
        println!("{}: {}", "warning".yellow().bold(), format!($($arg)*))
    }
}

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    steam_path: String,
    url_path: String,
}

fn main() {
    let cli = Cli::parse();

    let steam_path = Path::new(&cli.steam_path);
    let url_path = Path::new(&cli.url_path);

    // Do a path check (is checked later again to avoid race condition)

    if !steam_path.exists() {
        error!("Steam path does not exist");
    }

    if !url_path.exists() {
        error!("URL path does not exist");
    }

    // Look for and update all .url files in the URL path
    
    let mut downloaded = 0;

    let dir = match url_path.read_dir() {
        Ok(dir) => dir,
        Err(e) => error!("Failed to read URL path: {}", e),
    };

    for entry in dir {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                warn!("Failed to directory entry: {}", e);
                continue;
            },
        };

        // Skip non-files and non-.url files

        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let extension = match path.extension() {
            Some(extension) => extension,
            None => continue,
        };

        if extension != "url" {
            continue;
        }

        // Open and parse the .url file

        let mut file = match std::fs::File::open(&path) {
            Ok(file) => file,
            Err(e) => {
                warn!("Failed to open '{}': {}", path.display(), e);
                continue;
            },
        };
        
        let parse_option = ParseOption {
            enabled_escape: false,
            ..Default::default()
        };

        let mut conf = match Ini::read_from_opt(&mut file, parse_option) {
            Ok(conf) => conf,
            Err(e) => {
                warn!("Failed parse '{}': {}", path.display(), e);
                continue;
            },
        };

        // Get [InternetShortcut] section

        let section = match conf.section_mut(Some("InternetShortcut")) {
            Some(section) => section,
            None => {
                warn!("No [InternetShortcut] section in '{}'", path.display());
                continue;
            },
        };

        // Get url and icon file

        let url = match section.get("URL") {
            Some(url) => url,
            None => {
                warn!("No URL in '{}'", path.display());
                continue;
            },
        };
        
        let icon_file = match section.get("IconFile") {
            Some(icon_file) => icon_file,
            None => {
                warn!("No IconFile in '{}'", path.display());
                continue;
            },
        };
        
        // Skip non-steam urls

        if !url.starts_with("steam://rungameid/") {
            warn!("Skipping '{}' because it does not have a steam url attached to it", path.display());
            continue;
        }

        // Extract steam id and icon file name

        let steam_id = match url.strip_prefix("steam://rungameid/") {
            Some(steam_id) => steam_id,
            None => {
                warn!("Failed to extract steam id from '{}'", url);
                continue;
            },
        };

        let icon_file_path = Path::new(&icon_file);
        let icon_file_name = match icon_file_path.file_name() {
            Some(icon_file_name) => icon_file_name,
            None => {
                warn!("Failed to get icon file name from '{}'", icon_file);
                continue;
            },
        };
        let icon_file_name = match icon_file_name.to_str() {
            Some(icon_file_name) => icon_file_name,
            None => {
                warn!("Failed to convert icon file name to string: {:?}", icon_file_name);
                continue;
            },
        };
        
        let new_icon_path = steam_path.join(icon_file_name);
        
        // Check if the icon file already exists
        if new_icon_path.exists() {
            warn!("Skipping '{}' because '{}' already exists", path.display(), new_icon_path.display());
            continue;
        }

        let icon_url = format!("{}/{}/{}", STEAM_CDN, steam_id, icon_file_name);

        println!("{}: {}", "downloading".green().bold(), icon_url);

        let response = match reqwest::blocking::get(&icon_url) {
            Ok(response) => response,
            Err(e) => {
                warn!("Failed to get icon from '{}': {}", icon_url, e);
                continue;
            },
        };

        if !response.status().is_success() {
            warn!("Failed to get icon from '{}': {}", icon_url, response.status());
            continue;
        }

        let icon_data = match response.bytes() {
            Ok(icon_data) => icon_data,
            Err(e) => {
                warn!("Failed to get icon from '{}': {}", icon_url, e);
                continue;
            },
        };
        
        match std::fs::write(&new_icon_path, icon_data) {
            Ok(_) => (),
            Err(e) => {
                warn!("Failed to write icon to '{}': {}", new_icon_path.display(), e);
                continue;
            },
        }
        
        downloaded += 1;
    }
    
    if downloaded > 0 {
        println!("{}: {} icons have been downloaded", "done".blue().bold(), downloaded);
    } else {
        println!("{}", "Nothing has changed".yellow().bold());
    }
}
