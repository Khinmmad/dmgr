//! pactl backend — works with PipeWire-pulse and native PulseAudio.

use super::{detect_kind, AudioBackend, AudioDevice};
use std::process::Command;

pub struct Pactl;

impl AudioBackend for Pactl {
    fn name(&self) -> &'static str {
        "pactl"
    }

    fn outputs(&self) -> Vec<AudioDevice> {
        let def = default_of("get-default-sink");
        run(&["list", "sinks"])
            .map(|t| parse(&t, "Sink", &def))
            .unwrap_or_default()
    }

    fn inputs(&self) -> Vec<AudioDevice> {
        let def = default_of("get-default-source");
        run(&["list", "sources"])
            .map(|t| parse(&t, "Source", &def))
            .unwrap_or_default()
            .into_iter()
            .filter(|d| !d.name.ends_with(".monitor") && !d.description.starts_with("Monitor of"))
            .collect()
    }

    fn set_default_output(&self, id: &str) -> Result<(), String> {
        cmd(&["set-default-sink", id])
    }
    fn set_default_input(&self, id: &str) -> Result<(), String> {
        cmd(&["set-default-source", id])
    }
    fn set_volume(&self, id: &str, percent: u32) -> Result<(), String> {
        cmd(&["set-sink-volume", id, &format!("{}%", percent.min(150))])
    }
    fn set_mute(&self, id: &str, muted: bool) -> Result<(), String> {
        cmd(&["set-sink-mute", id, if muted { "1" } else { "0" }])
    }
}

fn default_of(c: &str) -> String {
    run(&[c]).map(|s| s.trim().to_string()).unwrap_or_default()
}

fn run(args: &[&str]) -> Option<String> {
    let out = Command::new("pactl").args(args).output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

fn cmd(args: &[&str]) -> Result<(), String> {
    let st = Command::new("pactl")
        .args(args)
        .status()
        .map_err(|e| format!("pactl not found: {e}"))?;
    if st.success() {
        Ok(())
    } else {
        Err(format!("pactl {} failed", args.join(" ")))
    }
}

fn parse(text: &str, kind: &str, default_name: &str) -> Vec<AudioDevice> {
    let prefix = format!("{kind} #");
    let mut out = Vec::new();
    let mut cur: Option<AudioDevice> = None;

    let flush = |c: &mut Option<AudioDevice>, v: &mut Vec<AudioDevice>| {
        if let Some(mut d) = c.take() {
            if !d.name.is_empty() {
                if d.description.is_empty() {
                    d.description = d.name.clone();
                }
                v.push(d);
            }
        }
    };

    for line in text.lines() {
        let t = line.trim();
        if t.starts_with(&prefix) {
            flush(&mut cur, &mut out);
            cur = Some(AudioDevice {
                index: t[prefix.len()..].trim().parse().unwrap_or(0),
                name: String::new(),
                description: String::new(),
                state: String::new(),
                muted: false,
                volume: None,
                is_default: false,
                kind: "Builtin".into(),
            });
        } else if let Some(d) = cur.as_mut() {
            if let Some(v) = t.strip_prefix("Name: ") {
                d.name = v.to_string();
                d.kind = detect_kind(v).to_string();
                d.is_default = v == default_name;
            } else if let Some(v) = t.strip_prefix("Description: ") {
                d.description = v.to_string();
            } else if let Some(v) = t.strip_prefix("State: ") {
                d.state = v.to_string();
            } else if let Some(v) = t.strip_prefix("Mute: ") {
                d.muted = v.trim() == "yes";
            } else if d.volume.is_none() && t.starts_with("Volume:") {
                d.volume = t
                    .split('/')
                    .map(str::trim)
                    .find(|s| s.ends_with('%'))
                    .and_then(|s| s.trim_end_matches('%').trim().parse().ok());
            }
        }
    }
    flush(&mut cur, &mut out);
    out
}

// ── Per-application streams (sink-inputs) ───────────────────────────────────

pub fn app_streams() -> Vec<super::AudioApp> {
    run(&["list", "sink-inputs"]).map(|t| parse_inputs(&t)).unwrap_or_default()
}

pub fn set_app_volume(index: u32, percent: u32) -> Result<(), String> {
    cmd(&[
        "set-sink-input-volume",
        &index.to_string(),
        &format!("{}%", percent.min(150)),
    ])
}

pub fn set_app_mute(index: u32, muted: bool) -> Result<(), String> {
    cmd(&[
        "set-sink-input-mute",
        &index.to_string(),
        if muted { "1" } else { "0" },
    ])
}

fn parse_inputs(text: &str) -> Vec<super::AudioApp> {
    let mut out: Vec<super::AudioApp> = Vec::new();
    let mut cur: Option<super::AudioApp> = None;

    let flush = |c: &mut Option<super::AudioApp>, v: &mut Vec<super::AudioApp>| {
        if let Some(mut a) = c.take() {
            if a.name.is_empty() {
                a.name = if a.media.is_empty() {
                    format!("Stream #{}", a.index)
                } else {
                    a.media.clone()
                };
            }
            v.push(a);
        }
    };

    for line in text.lines() {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("Sink Input #") {
            flush(&mut cur, &mut out);
            cur = Some(super::AudioApp {
                index: rest.trim().parse().unwrap_or(0),
                name: String::new(),
                media: String::new(),
                volume: None,
                muted: false,
            });
        } else if let Some(a) = cur.as_mut() {
            if let Some(v) = t.strip_prefix("Mute: ") {
                a.muted = v.trim() == "yes";
            } else if a.volume.is_none() && t.starts_with("Volume:") {
                a.volume = t
                    .split('/')
                    .map(str::trim)
                    .find(|s| s.ends_with('%'))
                    .and_then(|s| s.trim_end_matches('%').trim().parse().ok());
            } else if let Some(v) = t.strip_prefix("application.name = ") {
                a.name = v.trim().trim_matches('"').to_string();
            } else if let Some(v) = t.strip_prefix("media.name = ") {
                a.media = v.trim().trim_matches('"').to_string();
            }
        }
    }
    flush(&mut cur, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
Sink #49
\tState: RUNNING
\tName: alsa_output.pci-0000_0c_00.4.analog-stereo
\tDescription: Family 17h/19h HD Audio Analog Stereo
\tMute: no
\tVolume: front-left: 42000 /  65% / -9.33 dB,   front-right: 42000 /  65%
Sink #52
\tState: SUSPENDED
\tName: alsa_output.pci-0000_01_00.1.hdmi-stereo-extra3
\tDescription: GB206 HDMI Digital Stereo
\tMute: yes
\tVolume: front-left: 26214 /  40% / -23.65 dB
";

    #[test]
    fn parses_two_sinks_with_default_and_volume() {
        let default = "alsa_output.pci-0000_0c_00.4.analog-stereo";
        let sinks = parse(SAMPLE, "Sink", default);
        assert_eq!(sinks.len(), 2);

        let analog = &sinks[0];
        assert!(analog.is_default);
        assert_eq!(analog.state, "RUNNING");
        assert!(!analog.muted);
        assert_eq!(analog.volume, Some(65));
        assert_eq!(analog.kind, "Builtin");

        let hdmi = &sinks[1];
        assert!(!hdmi.is_default);
        assert!(hdmi.muted);
        assert_eq!(hdmi.volume, Some(40));
        assert_eq!(hdmi.kind, "Hdmi");
        assert_eq!(hdmi.description, "GB206 HDMI Digital Stereo");
    }

    #[test]
    fn empty_input_yields_nothing() {
        assert!(parse("", "Sink", "").is_empty());
    }

    const SINK_INPUTS: &str = "\
Sink Input #200
\tSink: 49
\tMute: no
\tVolume: front-left: 32768 /  50% / -18.06 dB,   front-right: 32768 /  50%
\tProperties:
\t\tapplication.name = \"Firefox\"
\t\tmedia.name = \"YouTube\"
Sink Input #201
\tMute: yes
\tVolume: front-left: 65536 / 100%
\tProperties:
\t\tmedia.name = \"Some game\"
";

    #[test]
    fn parses_sink_inputs_with_app_and_media() {
        let apps = parse_inputs(SINK_INPUTS);
        assert_eq!(apps.len(), 2);
        assert_eq!(apps[0].index, 200);
        assert_eq!(apps[0].name, "Firefox");
        assert_eq!(apps[0].media, "YouTube");
        assert_eq!(apps[0].volume, Some(50));
        assert!(!apps[0].muted);
        // no application.name → falls back to media.name
        assert_eq!(apps[1].name, "Some game");
        assert!(apps[1].muted);
        assert_eq!(apps[1].volume, Some(100));
    }
}
