//! Small terminal-UI helpers. We keep this deliberately minimal so
//! operators on headless servers with no unicode font support still get
//! readable output.

use console::style;
use dialoguer::{theme::ColorfulTheme, Confirm, Input};

pub fn banner() {
    let title = style("svx").bold().cyan();
    let tag = style("secure validator identity transfer").dim();
    eprintln!("{title}  {tag}");
}

pub fn section(label: &str) {
    eprintln!("\n{} {}", style("▸").bold().cyan(), style(label).bold());
}

pub fn info_line(msg: &str) {
    eprintln!("  {}", msg);
}

pub fn error_line(msg: &str) {
    eprintln!("  {} {}", style("✖").bold().red(), style(msg).red());
}

/// Show the SAS we computed locally and ask the operator to confirm the
/// far-side operator reads the SAME string.
pub fn confirm_sas(local: &str) -> bool {
    eprintln!(
        "  SAS (read to the sender and confirm they see the same): {}",
        style(local).bold().yellow()
    );
    Confirm::with_theme(&ColorfulTheme::default())
        .with_prompt("Does the sender's SAS match?")
        .default(false)
        .interact()
        .unwrap_or(false)
}

/// On the sender side: ask the operator to type what the receiver reads
/// out and compare. We compare case-insensitively and ignore spaces to
/// tolerate noisy voice channels.
pub fn prompt_sas_from_peer(local: &str) -> bool {
    eprintln!("  SAS on this machine: {}", style(local).bold().yellow());
    let typed: String = match Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Enter the SAS the receiver reads to you")
        .interact_text()
    {
        Ok(s) => s,
        Err(_) => return false,
    };
    normalize(&typed) == normalize(local)
}

fn normalize(s: &str) -> String {
    // Strip every non-alphabetic character so the comparison tolerates any
    // separators the operator might speak/type: dashes, spaces, commas, slashes.
    // e.g. "hunt-brisk-imp-cycle", "hunt brisk imp cycle", "HUNT, BRISK IMP/CYCLE"
    // all normalize to "huntbriskimpcycle".
    s.chars()
        .filter(|c| c.is_ascii_alphabetic())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::normalize;

    #[test]
    fn normalize_strips_dashes_spaces_and_case() {
        let canonical = "hunt-brisk-imp-cycle";
        assert_eq!(normalize(canonical), normalize("hunt brisk imp cycle"));
        assert_eq!(normalize(canonical), normalize("HUNT BRISK IMP CYCLE"));
        assert_eq!(normalize(canonical), normalize("Hunt, Brisk Imp / Cycle"));
        assert_eq!(
            normalize(canonical),
            normalize("  hunt  brisk  imp  cycle  ")
        );
    }

    #[test]
    fn normalize_rejects_different_words() {
        assert_ne!(
            normalize("hunt-brisk-imp-cycle"),
            normalize("hunt-brisk-imp-cycl")
        );
        assert_ne!(
            normalize("hunt-brisk-imp-cycle"),
            normalize("hunt-brisk-imp-xycle")
        );
    }
}
