use std::fs::File;
use std::io::Write;
use std::path::Path;
use svg::{
    svg_draw_barres, svg_draw_bg, svg_draw_min_fret, 
    svg_draw_note, svg_draw_title, svg_draw_muted_string, svg_draw_open_string
};
use tera::{Context as TeraContext, Tera};
use types::{Chord, GuitarString, Hand};
use utils::{get_filename, get_palette};

mod svg;
pub mod types;
mod utils;

// ♭ \u266D
// ♯ \u266F
// natural ♮ \u266E
// dim o U+E870
// aug + U+E872 +

fn generate_svg(chord_settings: Chord) -> Result<String, Box<dyn std::error::Error>> {
    let string_space = 40;
    let margin = 30;

    let palette = get_palette(chord_settings.mode);

    // var for switching between handedness
    let total_strings = 5;

    let lowest_fret: &i32 = chord_settings
        .frets
        .iter()
        .filter(|fret| **fret > 0)
        .min()
        .unwrap_or(&0);

    let show_nut = (chord_settings.frets.contains(&0) && lowest_fret < &3)
        || chord_settings.frets.contains(&1);
    let nut_width = if show_nut { 9 } else { 2 };
    let nut_shape = if show_nut { "round" } else { "butt" };

    let mut muted = String::new();
    for (i, fret) in chord_settings.frets.iter().enumerate() {
        if *fret == -1 {
            let string: GuitarString = if chord_settings.hand == Hand::Right {
                i.into()
            } else {
                (total_strings - i).into()
            };
            muted += &svg_draw_muted_string(string, &string_space, &palette);
        }
    }

    let mut open = String::new();
    let show_open = chord_settings.barres.is_none();

    if show_open {
        for (i, fret) in chord_settings.frets.iter().enumerate() {
            if *fret == 0 {
                let string: GuitarString = if chord_settings.hand == Hand::Right {
                    i.into()
                } else {
                    (total_strings - i).into()
                };
                open += &svg_draw_open_string(string, &string_space, &palette);
            }
        }
    }
    
    let mut notes = "".to_string();
    for (i, note) in chord_settings.frets.iter().enumerate() {
        if *note > 0 {
            let string: GuitarString = if chord_settings.hand == Hand::Right {
                i.into()
            } else {
                (total_strings - i).into()
            };
            notes += &svg_draw_note(note, string, &string_space, lowest_fret, &palette);
        }
    }

    let mut min_fret_marker = "".to_string();
    if *lowest_fret > 2 || *lowest_fret > 1 && !show_nut {
        min_fret_marker = svg_draw_min_fret(lowest_fret, &string_space, &palette);
    }

    let chord_title = svg_draw_title(&chord_settings, &palette);
    // if barre
    // for each barre
    let barres = match chord_settings.barres {
        Some(barres) => svg_draw_barres(
            &barres[0],
            &chord_settings.frets,
            &string_space,
            lowest_fret,
            &palette,
        ),
        None => String::from(""),
    };

    let mut context = TeraContext::new();
    context.insert("name", &chord_title);
    context.insert("padding", &margin);
    context.insert("nutWidth", &nut_width);
    context.insert("nutShape", &nut_shape);
    context.insert("notes", &notes);
    context.insert("minFret", &min_fret_marker);
    context.insert("muted", &muted);
    context.insert("open", &open);
    context.insert("foreground", &palette.fg);
    context.insert(
        "background",
        &svg_draw_bg(chord_settings.use_background, &palette),
    );
    context.insert("barres", &barres);

    match Tera::one_off(include_str!("../templates/chord.svg"), &context, false) {
        Ok(result) => Ok(result),
        Err(e) => {
            println!("{:?}", e);
            Err(Box::new(e))
        }
    }
}

pub fn render_svg(
    chord_settings: Chord,
    output_dir: &str,
) -> Result<u64, Box<dyn std::error::Error>> {
    let hashed_title = get_filename(&chord_settings);

    match generate_svg(chord_settings) {
        Ok(result) => {
            let path = Path::new(output_dir).join(format!("{}.svg", hashed_title));
            let mut output = File::create(path)?;
            write!(output, "{}", result)?;
            Ok(hashed_title)
        }

        Err(e) => {
            println!("Failed to create SVG: {:?}", e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        generate_svg,
        types::{Chord, Mode},
    };
    use std::fs;
    use std::path::Path;

    #[test]
    fn generates_svg_for_barre3_major() {
        let shape = [-1, 0, 2, 2, 2, 0];
        let fret = 3;
        let frets: Vec<i32> = shape.iter()
            .map(|&f| {
                if f < 0 {
                    -1 // preserve mute
                } else if f == 0 {
                    fret // open string → fret 5
                } else {
                    fret + f
                }
            })
            .collect();
        let barre_fret = fret;

        let title = String::from("C");
        let suffix = String::from("maj");
        let chord = Chord {
            frets,
            title: Some(&title),
            suffix: Some(&suffix),
            mode: Mode::Light,
            barres: Some(vec![barre_fret]),
            use_background: true,
            ..Default::default()
        };

        let svg = generate_svg(chord).unwrap();

        // Basic checks
        assert!(svg.contains("<svg"));
        assert!(svg.contains("circle"));
        assert!(svg.contains("C"));

        // Save to file for inspection
        let out_path = Path::new("fixtures/output/barre3_maj.svg");
        fs::create_dir_all(out_path.parent().unwrap()).unwrap();
        fs::write(out_path, svg).unwrap();
    }

    #[test]
    fn generates_svg_for_c_open() {
        let shape = [-1, 3, 2, 0, 1, 0]; // Standard C major open shape
        let frets: Vec<i32> = shape.iter()
            .map(|&f| if f < 0 { -1 } else { f })
            .collect();

        let title = String::from("C");
        let suffix = String::from("maj");

        let chord = Chord {
            frets,
            title: Some(&title),
            suffix: Some(&suffix),
            mode: Mode::Light,
            use_background: true,
            barres: None, 
            ..Default::default()
        };

        let svg = generate_svg(chord).unwrap();

        // Sanity checks
        assert!(svg.contains("<svg"));
        assert!(svg.contains("circle")); // At least some fretted notes
        assert!(svg.contains("C"));
        assert!(svg.contains(">0<")); // Open string labels

        // Save to file
        let out_path = Path::new("fixtures/output/cmaj_open.svg");
        fs::create_dir_all(out_path.parent().unwrap()).unwrap();
        fs::write(out_path, svg).unwrap();
    }
}
