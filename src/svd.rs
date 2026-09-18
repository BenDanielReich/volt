//! Import CMSIS-SVD (or similar) XML into Volt `reg` declarations.

pub fn svd_to_volt(xml: &str) -> Result<String, String> {
    let mut out = String::from("module svd\n\n");
    let peripherals = tags(xml, "peripheral");
    if peripherals.is_empty() {
        return Err("no <peripheral> elements found".into());
    }
    for periph in peripherals {
        let pname = first_direct_name(&periph).unwrap_or("PERIPH");
        let base = parse_int(&inner_of(&periph, "baseAddress").unwrap_or_else(|| "0".into())).unwrap_or(0);
        out.push_str(&format!("// peripheral {pname} @ 0x{base:X}\n"));
        for reg in tags(&periph, "register") {
            let rname = first_direct_name(&reg).unwrap_or("REG");
            let off = parse_int(&inner_of(&reg, "addressOffset").unwrap_or_else(|| "0".into())).unwrap_or(0);
            let size = parse_int(&inner_of(&reg, "size").unwrap_or_else(|| "8".into())).unwrap_or(8);
            let ty = match size {
                8 => "u8",
                16 => "u16",
                32 => "u32",
                64 => "u64",
                n if n <= 8 => "u8",
                n if n <= 16 => "u16",
                n if n <= 32 => "u32",
                _ => "u64",
            };
            let addr = base + off;
            let fields = tags(&reg, "field");
            if fields.is_empty() {
                out.push_str(&format!("reg {ty} {pname}_{rname} @ 0x{addr:X}\n"));
                continue;
            }
            out.push_str(&format!("reg {ty} {pname}_{rname} @ 0x{addr:X} {{\n"));
            for field in fields {
                let fname = first_direct_name(&field).unwrap_or("bit");
                let fname = sanitize(fname);
                let start = parse_int(
                    &inner_of(&field, "bitOffset")
                        .or_else(|| inner_of(&field, "lsb"))
                        .unwrap_or_else(|| "0".into()),
                )
                .unwrap_or(0);
                let width = parse_int(&inner_of(&field, "bitWidth").unwrap_or_else(|| "1".into())).unwrap_or(1);
                if width <= 1 {
                    out.push_str(&format!("    {fname}: {start},\n"));
                } else {
                    let end = start + width - 1;
                    out.push_str(&format!("    {fname}: {start}..{end},\n"));
                }
            }
            out.push_str("}\n");
        }
        out.push('\n');
    }
    Ok(out)
}

fn tags(xml: &str, name: &str) -> Vec<String> {
    let open = format!("<{name}");
    let close = format!("</{name}>");
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find(&open) {
        let after = &rest[start + open.len()..];
        let body_start = match after.find('>') {
            Some(i) => start + open.len() + i + 1,
            None => break,
        };
        if after.trim_start().starts_with('/') {
            rest = &rest[body_start..];
            continue;
        }
        if let Some(end) = rest[body_start..].find(&close) {
            out.push(rest[body_start..body_start + end].to_string());
            rest = &rest[body_start + end + close.len()..];
        } else {
            break;
        }
    }
    out
}

fn inner_of(xml: &str, name: &str) -> Option<String> {
    tags(xml, name).into_iter().next()
}

/// The first `<name>` that is a direct-ish child, not nested in a descendant
/// register/field. We take the first `<name>` in the snippet.
fn first_direct_name(xml: &str) -> Option<&str> {
    let start = xml.find("<name>")? + 6;
    let end = xml[start..].find("</name>")?;
    Some(xml[start..start + end].trim())
}

fn parse_int(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else {
        s.parse().ok()
    }
}

fn sanitize(name: &str) -> String {
    let mut out = String::new();
    for (i, ch) in name.chars().enumerate() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            if i == 0 && ch.is_ascii_digit() {
                out.push('_');
            }
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "_bit".into()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imports_register_and_field() {
        let xml = r#"
            <device>
              <peripheral>
                <name>PORTB</name>
                <baseAddress>0x20</baseAddress>
                <register>
                  <name>PORT</name>
                  <addressOffset>0x05</addressOffset>
                  <size>8</size>
                  <fields>
                    <field>
                      <name>PB5</name>
                      <bitOffset>5</bitOffset>
                      <bitWidth>1</bitWidth>
                    </field>
                  </fields>
                </register>
              </peripheral>
            </device>
        "#;
        let volt = svd_to_volt(xml).unwrap();
        assert!(volt.contains("reg u8 PORTB_PORT @ 0x25"));
        assert!(volt.contains("PB5: 5"));
    }
}
