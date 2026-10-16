// `cargo xtask decode` -- run captured radio payloads through ropk-proto on the
// host and print what they mean. No board needed.
//
// Input is forgiving so a line can be pasted straight from a defmt log:
//   f1 2e 06 45 ...            spaced bytes
//   f12e0645...                one hex run
//   0xf1, 0x2e, 0x06           C-style
//   ... Payload  : [f1, 2e]    defmt `{=[u8]:02x}` (only the [...] part is used)
// Lines starting with '#' and blank lines are skipped.

use std::fmt::Write as _;

use ropk_proto::frame::{Reply, parse_reply};
use ropk_proto::input::{ControllerInput, Imu, Trackpad, trigger_u8};
use ropk_proto::report::{Report, parse_report};
use ropk_proto::sc_controller_buttons::Buttons;
use ropk_proto::tlv::{TAG_BLOB, TAG_REPORT, TAG_STATUS};

/// Radio RAM layout is [LEN][S1][payload..]; ScRadioData strips the first two.
/// `Raw` accepts the full RAM dump (e.g. a C `rfrx` print) and strips them here.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Framing {
    Payload,
    Raw,
}

/// Parse one line of hex. `Ok(None)` for comments/blank lines.
pub fn parse_hex_line(line: &str) -> Result<Option<Vec<u8>>, String> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return Ok(None);
    }
    // defmt slice formatting: keep only what's inside the last [...]
    let body = match (line.rfind('['), line.rfind(']')) {
        (Some(a), Some(b)) if a < b => &line[a + 1..b],
        _ => line,
    };

    let mut out = Vec::new();
    for tok in body.split(|c: char| c.is_whitespace() || c == ',') {
        let tok = tok.trim();
        if tok.is_empty() {
            continue;
        }
        let tok = tok
            .strip_prefix("0x")
            .or_else(|| tok.strip_prefix("0X"))
            .unwrap_or(tok);
        if tok.len() == 1 {
            // defmt prints single digits unpadded only without :02x; accept it
            out.push(u8::from_str_radix(tok, 16).map_err(|_| format!("bad hex '{tok}'"))?);
            continue;
        }
        if tok.len() % 2 != 0 {
            return Err(format!("odd-length hex token '{tok}'"));
        }
        for i in (0..tok.len()).step_by(2) {
            let pair = &tok[i..i + 2];
            out.push(u8::from_str_radix(pair, 16).map_err(|_| format!("bad hex '{pair}'"))?);
        }
    }
    if out.is_empty() {
        return Ok(None);
    }
    Ok(Some(out))
}

/// Strip [LEN][S1] the way ScRadioData::from_buf does, so a raw dump decodes
/// identically to what the firmware would see.
pub fn strip_radio_header(raw: &[u8]) -> Result<(u8, &[u8]), String> {
    if raw.len() < 2 {
        return Err(format!("raw dump needs [LEN][S1], got {} bytes", raw.len()));
    }
    let len = raw[0] as usize;
    let avail = raw.len() - 2;
    if len > avail {
        return Err(format!(
            "LEN byte says {len} but only {avail} payload bytes follow"
        ));
    }
    Ok((raw[1], &raw[2..2 + len]))
}

fn hex(b: &[u8]) -> String {
    b.iter()
        .map(|x| format!("{x:02x}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn rtype_name(rtype: u8) -> &'static str {
    match rtype {
        0xE1 => "E1 host frame / discovery beacon",
        0xE2 | 0xE3 => "E3 input poll",
        0xE7 => "E7 wake / version",
        0xF0..=0xFF => "F-type",
        _ => "unknown",
    }
}

fn tag_name(tag: u8) -> &'static str {
    match tag {
        TAG_STATUS => "status",
        TAG_BLOB => "blob",
        TAG_REPORT => "report",
        _ => "unknown tag",
    }
}

fn buttons_line(b: Buttons) -> String {
    let mut s = format!("{:#010x}", b.bits());
    let names: Vec<String> = b.iter().map(|x| format!("{x:?}")).collect();
    if names.is_empty() {
        s.push_str("  (none)");
    } else {
        s.push_str("  ");
        s.push_str(&names.join(" "));
    }
    let unknown = b.remove(Buttons::PHYSICAL);
    if !unknown.is_empty() {
        let _ = write!(s, "  +non-physical {:#010x}", unknown.bits());
    }
    s
}

fn pad_line(p: Trackpad) -> String {
    format!("x {:6}  y {:6}  press {:6}", p.x, p.y, p.press)
}

fn imu_line(imu: Option<Imu>) -> String {
    match imu {
        None => "none (short report: no IMU tail)".into(),
        Some(i) => format!(
            "ts {:#010x}  accel {:?}  gyro {:?}",
            i.timestamp, i.accel, i.gyro
        ),
    }
}

fn input_block(out: &mut String, indent: &str, i: &ControllerInput) {
    let _ = writeln!(out, "{indent}buttons   {}", buttons_line(i.buttons));
    let _ = writeln!(
        out,
        "{indent}triggers  L {:#06x} ({:3})  R {:#06x} ({:3})",
        i.triggers.l,
        trigger_u8(i.triggers.l),
        i.triggers.r,
        trigger_u8(i.triggers.r)
    );
    let _ = writeln!(
        out,
        "{indent}sticks    L ({:6}, {:6})  R ({:6}, {:6})",
        i.sticks.lx, i.sticks.ly, i.sticks.rx, i.sticks.ry
    );
    let _ = writeln!(out, "{indent}l pad     {}", pad_line(i.l_trackpad));
    let _ = writeln!(out, "{indent}r pad     {}", pad_line(i.r_trackpad));
    let _ = writeln!(out, "{indent}imu       {}", imu_line(i.imu));
}

/// Human-readable decode of one payload ([rtype][body..]).
pub fn describe(payload: &[u8]) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "payload ({} B): {}", payload.len(), hex(payload));

    let reply = match parse_reply(payload) {
        Ok(r) => r,
        Err(e) => {
            let _ = writeln!(out, "  FRAME ERROR: {e:?}");
            return out;
        }
    };
    let alive = if reply.is_link_alive() {
        "link alive"
    } else {
        "NOT ours"
    };

    match reply {
        Reply::NotOurs { rtype } => {
            let _ = writeln!(
                out,
                "  {rtype:#04x} {}  ({alive}: another puck's traffic)",
                rtype_name(rtype)
            );
            if rtype == 0xE1 {
                e1_block(&mut out, payload);
            }
        }
        Reply::Disconnect => {
            let _ = writeln!(out, "  F2 disconnect / power-off  ({alive})");
        }
        Reply::Status { version, .. } => {
            let _ = writeln!(out, "  F3 status  version {version:#04x}  ({alive})");
        }
        Reply::OtherReply { rtype, raw } => {
            let _ = writeln!(
                out,
                "  {rtype:#04x} undecoded F-type  ({alive})  {}",
                hex(raw)
            );
        }
        Reply::Input(frame) => {
            let _ = writeln!(out, "  F1 input  ({alive})");
            // walk the TLVs ourselves (not reports()) so non-report tags and
            // their offsets show up too
            let mut at = 0usize;
            let mut n = 0usize;
            for item in frame.tlvs() {
                match item {
                    Err(e) => {
                        let _ = writeln!(out, "  TLV ERROR: {e:?}  (walk stops here)");
                        break;
                    }
                    Ok(t) => {
                        let _ = writeln!(
                            out,
                            "  tlv[{n}] @{at:<3} tag {:#04x} {:<11} len {}",
                            t.tag,
                            tag_name(t.tag),
                            t.value.len()
                        );
                        if t.tag == TAG_REPORT {
                            report_block(&mut out, t.value);
                        } else {
                            let _ = writeln!(out, "      {}", hex(t.value));
                        }
                        at += 2 + t.value.len();
                        n += 1;
                    }
                }
            }
            if n == 0 {
                let _ = writeln!(out, "  (no records)");
            }
        }
    }
    out
}

/// E1 host frame, payload indices (PROTOCOL.md §6 minus the [LEN][S1] header):
///   [0] E1  [1..5] proteus_uuid LE  [5..9] ibex_uuid LE  [9] session channel
///   [10..13] zero  [13..17] session base (stored form)  [17] session prefix
pub const E1_LEN: usize = 18;

fn e1_block(out: &mut String, p: &[u8]) {
    if p.len() < E1_LEN {
        let _ = writeln!(
            out,
            "      E1 too short for host-frame layout ({} < {E1_LEN} B)",
            p.len()
        );
        return;
    }
    let le = |i: usize| u32::from_le_bytes([p[i], p[i + 1], p[i + 2], p[i + 3]]);
    let _ = writeln!(
        out,
        "      proteus_uuid    {:#010x}  (bytes {})  = bond rec[0..4]",
        le(1),
        hex(&p[1..5])
    );
    let _ = writeln!(
        out,
        "      ibex_uuid       {:#010x}  (bytes {})  = bond rec[4..8]",
        le(5),
        hex(&p[5..9])
    );
    let _ = writeln!(
        out,
        "      session channel {}  ({} MHz)",
        p[9],
        2400 + p[9] as u32
    );
    let _ = writeln!(
        out,
        "      session base    {}  (stored form; bitrev each byte for on-air)",
        hex(&p[13..17])
    );
    let _ = writeln!(out, "      session prefix  {:#04x}", p[17]);
    if p[10..13] != [0, 0, 0] {
        let _ = writeln!(
            out,
            "      note: bytes 10..13 = {} (expected 00 00 00)",
            hex(&p[10..13])
        );
    }
    if p.len() > E1_LEN {
        let _ = writeln!(
            out,
            "      note: {} trailing byte(s) {}",
            p.len() - E1_LEN,
            hex(&p[E1_LEN..])
        );
    }
}

fn report_block(out: &mut String, value: &[u8]) {
    match parse_report(value) {
        Err(e) => {
            let _ = writeln!(out, "      REPORT ERROR: {e:?}");
        }
        Ok(Report::Input(r)) => {
            let _ = writeln!(
                out,
                "      report {:#04x} input  seq {}  ({} B)",
                r.id,
                r.seq,
                r.raw.len()
            );
            input_block(out, "        ", &r.input);
        }
        Ok(Report::Status(s)) => {
            let _ = writeln!(
                out,
                "      report 0x43 status  charge_state {}  battery {}%",
                s.charge_state, s.battery_pct
            );
        }
        Ok(Report::StatusEvent(raw)) => {
            let _ = writeln!(out, "      report 0x44 status event  {}", hex(raw));
        }
        Ok(Report::Unknown { id, raw }) => {
            let _ = writeln!(out, "      report {id:#04x} UNKNOWN id  {}", hex(raw));
        }
    }
}

/// Decode every payload line in `text`. Returns (output, payloads, had_error).
pub fn decode_text(text: &str, framing: Framing) -> (String, usize, bool) {
    let mut out = String::new();
    let mut count = 0;
    let mut had_error = false;
    for (lineno, line) in text.lines().enumerate() {
        let bytes = match parse_hex_line(line) {
            Ok(None) => continue,
            Ok(Some(b)) => b,
            Err(e) => {
                let _ = writeln!(out, "line {}: {e}\n", lineno + 1);
                had_error = true;
                continue;
            }
        };
        count += 1;
        let payload: &[u8] = match framing {
            Framing::Payload => &bytes,
            Framing::Raw => match strip_radio_header(&bytes) {
                Ok((s1, p)) => {
                    let _ = writeln!(
                        out,
                        "radio header: LEN {}  S1 {s1:#04x} (PID {})",
                        p.len(),
                        (s1 >> 1) & 3
                    );
                    p
                }
                Err(e) => {
                    let _ = writeln!(out, "line {}: {e}\n", lineno + 1);
                    had_error = true;
                    continue;
                }
            },
        };
        let d = describe(payload);
        if d.contains("ERROR") {
            had_error = true;
        }
        out.push_str(&d);
        out.push('\n');
    }
    (out, count, had_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_formats() {
        let want = Some(vec![0xF1, 0x2E, 0x06]);
        assert_eq!(parse_hex_line("f1 2e 06").unwrap(), want);
        assert_eq!(parse_hex_line("F12E06").unwrap(), want);
        assert_eq!(parse_hex_line("0xf1, 0x2e, 0x06").unwrap(), want);
        assert_eq!(
            parse_hex_line("0.123 INFO  Payload  : [f1, 2e, 06]").unwrap(),
            want
        );
        assert_eq!(parse_hex_line("[f1, 2e, 6]").unwrap(), want);
    }

    #[test]
    fn hex_skips_comments_and_blank() {
        assert_eq!(parse_hex_line("").unwrap(), None);
        assert_eq!(parse_hex_line("   ").unwrap(), None);
        assert_eq!(parse_hex_line("# f1 2e").unwrap(), None);
        assert_eq!(parse_hex_line("[]").unwrap(), None);
    }

    #[test]
    fn hex_rejects_garbage() {
        assert!(parse_hex_line("f1 zz").is_err());
        assert!(parse_hex_line("f12").is_err());
    }

    #[test]
    fn raw_header_stripped_like_firmware() {
        assert_eq!(
            strip_radio_header(&[3, 0x02, 0xF2, 0, 0]).unwrap(),
            (0x02, &[0xF2, 0, 0][..])
        );
        assert!(strip_radio_header(&[5, 0, 0xF2]).is_err());
        assert!(strip_radio_header(&[1]).is_err());
    }

    #[test]
    fn describe_names_buttons_and_flags_errors() {
        let mut rep = vec![0u8; 30];
        rep[0] = 0x45;
        rep[1] = 9;
        rep[2] = 0x01; // A
        let mut p = vec![0xF1, rep.len() as u8, TAG_REPORT];
        p.extend_from_slice(&rep);
        let d = describe(&p);
        assert!(d.contains("F1 input"), "{d}");
        assert!(d.contains("seq 9"), "{d}");
        assert!(d.contains(" A"), "{d}");
        assert!(d.contains("short report: no IMU"), "{d}");
        assert!(!d.contains("ERROR"), "{d}");

        assert!(describe(&[0xF3, 1]).contains("FRAME ERROR"));
        assert!(describe(&[0xF1, 9, 6, 0x45]).contains("TLV ERROR"));
        assert!(describe(&[0xE1, 0]).contains("NOT ours"));
    }

    #[test]
    fn e1_fields_are_broken_out() {
        let p = [
            0xE1, 0x78, 0x56, 0x34, 0x12, 0xEF, 0xCD, 0xAB, 0x90, 0x2A, 0, 0, 0, 0x11, 0x22, 0x33,
            0x44, 0x55,
        ];
        let d = describe(&p);
        assert!(d.contains("proteus_uuid    0x12345678"), "{d}");
        assert!(d.contains("ibex_uuid       0x90abcdef"), "{d}");
        assert!(d.contains("session channel 42  (2442 MHz)"), "{d}");
        assert!(d.contains("session base    11 22 33 44"), "{d}");
        assert!(d.contains("session prefix  0x55"), "{d}");
        assert!(!d.contains("note:"), "{d}");
        assert!(describe(&p[..10]).contains("too short"));
    }

    #[test]
    fn decode_text_counts_and_flags() {
        let text = "# comment\nf2\n\n0x12, 0x02, 0xf2\nzz\n";
        let (_, n, err) = decode_text(text, Framing::Payload);
        assert_eq!(n, 2);
        assert!(err); // the 'zz' line
    }
}
