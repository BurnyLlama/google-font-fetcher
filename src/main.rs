use inline_colorization::*;
use std::env;

use crate::{
    exit_codes::EXIT_CODE_INVALID_FONT_NAME,
    font_manifest::{FontManifest, get_font_base_path},
};

mod exit_codes;
mod font_manifest;

fn main() {
    // The first argument is the action to perform, valid actions: help, fetch
    let action = env::args().nth(1).unwrap_or("help".to_string());
    let args = env::args().skip(2).collect::<Vec<_>>();

    if action != "fetch" {
        println!("{color_blue}{style_bold}Usage:{color_reset}{style_reset}");
        println!(
            "{color_bright_black}fonty {color_blue}fetch <font1> {color_bright_blue}[font2] [font3] [...]{color_reset}"
        );
        println!(
            "{color_yellow}->{color_reset} Fetches the specified fonts from Google Fonts. (Specify at least one, but multiple is posssible.)"
        );
        println!(
            "{color_yellow}->{color_reset} If a font has spaces in its name, remember to quote or escape the font name."
        );
        println!("{color_bright_black}fonty {color_blue}help{color_reset}");
        println!("{color_yellow}->{color_reset} Prints this help message.");
        let fonty_base_dir = get_font_base_path();
        println!(
            "\n{color_yellow}->{color_reset} Font base dir {color_white}(installation dir){color_reset}: {color_blue}'{}'{color_reset}",
            fonty_base_dir
        );
        println!(
            "{color_yellow}->{color_reset} Change the font base dir with the {color_blue}$FONTY_BASE_PATH{color_reset} environment variable."
        );
        std::process::exit(0);
    }

    if args.is_empty() {
        println!("{color_red}ERROR:{color_reset} No fonts specified!");
        std::process::exit(EXIT_CODE_INVALID_FONT_NAME);
    }

    let invalid_fonts = args
        .iter()
        .filter(|&arg| !FontManifest::check_if_valid_font(arg))
        .collect::<Vec<_>>();

    if !invalid_fonts.is_empty() {
        println!(
            "{color_red}ERROR:{color_reset} The following fonts are invalid: {color_blue}'{}'{color_reset}",
            invalid_fonts // I don't even know ... this seems needed though! :)
                .iter()
                .map(|x| x.as_str())
                .collect::<Vec<_>>()
                .join(&format!("'{color_bright_black}, {color_blue}'"))
        );
        std::process::exit(EXIT_CODE_INVALID_FONT_NAME);
    }

    println!(
        "{color_cyan}INFO:{color_reset} Will download the following fonts: {color_blue}'{}'{color_reset}",
        args.join(&format!("'{color_bright_black}, {color_blue}'"))
    );

    let fonty_base_dir = get_font_base_path();
    println!(
        "{color_cyan}INFO:{color_reset} Font base dir {color_white}(installation dir){color_reset}: {color_blue}'{}'{color_reset}",
        fonty_base_dir
    );

    let font_manifest = {
        let font_manifest = FontManifest::fetch(args.iter().map(|s| s.as_str()).collect()).unwrap();

        // If there is only one pending font download, prepend a directory with the name of the font,
        // so all font files end up in their own sub directory.
        if args.len() == 1 {
            font_manifest.prepand_path_to_files(&args[0].replace(" ", "_"))
        } else {
            font_manifest
        }
    };

    println!(
        "{color_cyan}INFO:{color_reset} Writing text files... {color_white}(Licenes, READMEs, etc.){color_reset}"
    );
    font_manifest.write_files();

    println!("{color_cyan}INFO:{color_reset} Downloading font files...");
    font_manifest.fetch_files_from_refs();

    println!("{color_cyan}INFO:{color_reset} All downloads {color_green}DONE{color_reset}!");
}
