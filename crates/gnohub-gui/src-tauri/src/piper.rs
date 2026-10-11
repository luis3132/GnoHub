//! Dibujos de dispositivos de Piper. Si Piper está instalado, sus SVG (unos
//! 65 modelos) se leen de su archivo de recursos en tiempo de ejecución; no se
//! copian al proyecto. Siguen una convención que GnoHub también usa: el
//! elemento `buttonN` marca dónde va la etiqueta del botón N de libratbag,
//! `buttonN-leader` es su línea guía y `buttonN-path` la pieza física
//! (igual con `ledN`).

use tokio::process::Command;

const RESOURCE_FILES: [&str; 2] = [
    "/usr/share/piper/piper.gresource",
    "/usr/local/share/piper/piper.gresource",
];
const PREFIX: &str = "/org/freedesktop/Piper/svgs";

async fn extract(file: &str, path: &str) -> Option<String> {
    let out = Command::new("gresource")
        .args(["extract", file, path])
        .output()
        .await
        .ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Nombre del SVG para este VID:PID según `svg-lookup.ini` de Piper.
fn lookup(ini: &str, vendor: u16, product: u16) -> Option<String> {
    let wanted = [
        format!("usb:{vendor:04x}:{product:04x}"),
        format!("bluetooth:{vendor:04x}:{product:04x}"),
    ];
    let mut matches = false;
    for line in ini.lines().map(str::trim) {
        if line.starts_with('[') {
            matches = false;
        } else if let Some(list) = line.strip_prefix("DeviceMatch=") {
            matches = list
                .split(';')
                .any(|m| wanted.iter().any(|w| m.eq_ignore_ascii_case(w)));
        } else if let Some(svg) = line.strip_prefix("Svg=")
            && matches
        {
            return Some(svg.trim().to_owned());
        }
    }
    None
}

pub async fn svg_for(vendor: u16, product: u16) -> Option<String> {
    for file in RESOURCE_FILES {
        if !std::path::Path::new(file).is_file() {
            continue;
        }
        let ini = extract(file, &format!("{PREFIX}/svg-lookup.ini")).await?;
        let name = lookup(&ini, vendor, product)?;
        return extract(file, &format!("{PREFIX}/{name}")).await;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_svg_by_usb_id() {
        let ini = "[A]\nDeviceMatch=usb:046d:c08b;usb:046d:c332\nSvg=logitech-g502.svg\n\n[B]\nDeviceMatch=usb:1532:0084\nSvg=razer.svg\n";
        assert_eq!(
            lookup(ini, 0x046d, 0xc332).as_deref(),
            Some("logitech-g502.svg")
        );
        assert_eq!(lookup(ini, 0x1532, 0x0084).as_deref(), Some("razer.svg"));
        assert_eq!(lookup(ini, 0x1, 0x2), None);
    }
}

#[cfg(test)]
mod live {
    #[tokio::test]
    #[ignore]
    async fn live_piper_g502() {
        let svg = super::svg_for(0x046d, 0xc08b)
            .await
            .expect("Piper con dibujo del G502");
        assert!(svg.contains("id=\"button5\"") && svg.contains("id=\"button5-leader\""));
    }
}
