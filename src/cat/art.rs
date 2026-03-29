// Cat pixel art frames using Unicode half-block characters.
// Color hints in comments — actual coloring is applied at render time.
//
// Palette guide:
//   Body:  warm orange/ginger  (#E8A04C)
//   Dark:  dark brown stripes  (#8B5E3C)
//   Light: cream belly/muzzle  (#FFF0D4)
//   Eyes:  bright green         (#44DD44)
//   Nose:  pink                 (#FF9999)
//   Bg:    transparent (space)

// ─────────────────────────────────────────────
//  IDLE FRAMES (4 frames — breathing + tail)
// ─────────────────────────────────────────────

/// Idle frame 0: tail curled right, normal breathing
pub const IDLE_0: &[&str] = &[
    //         1111111111222222
    // 1234567890123456789012345
    r"        ╱╲     ╱╲        ", // ears
    r"       ╱██╲   ╱██╲       ", // inner ears
    r"      ╱████╲_╱████╲      ", // ear base
    r"     ┌──────────────┐    ", // head top
    r"     │  ◕        ◕  │    ", // eyes
    r"     │      ▼▼      │    ", // nose
    r"     │  ╱══╗██╔══╲  │    ", // mouth / muzzle
    r"     │ │ ═══╝╚═══ │ │    ", // whiskers
    r"     └──┬────────┬──┘    ", // chin
    r"      ┌─┘ ░░░░░░ └─┐    ", // neck
    r"     ┌┘  ░░░░░░░░  └┐   ", // chest
    r"     │  ░░░░░░░░░░  │   ", // body
    r"     │ ░░░░░░░░░░░░ │   ", // body
    r"     └─┬──┘    └──┬─┘   ", // paws
    r"       └──┘    └──┘  ~  ", // feet + tail curled right
];

/// Idle frame 1: tail up-right, slight inhale (body wider by 1 char)
pub const IDLE_1: &[&str] = &[
    r"        ╱╲     ╱╲        ",
    r"       ╱██╲   ╱██╲       ",
    r"      ╱████╲_╱████╲      ",
    r"     ┌──────────────┐    ",
    r"     │  ◕        ◕  │    ",
    r"     │      ▼▼      │    ",
    r"     │  ╱══╗██╔══╲  │    ",
    r"     │ │ ═══╝╚═══ │ │    ",
    r"     └──┬────────┬──┘    ",
    r"      ┌─┘ ░░░░░░ └─┐    ",
    r"    ┌─┘  ░░░░░░░░  └─┐  ",
    r"    │  ░░░░░░░░░░░░  │  ",
    r"    │ ░░░░░░░░░░░░░░ │  ",
    r"    └─┬──┘      └──┬─┘  ",
    r"      └──┘      └──┘ ⌇  ",
];

/// Idle frame 2: tail curled left, exhale back to normal
pub const IDLE_2: &[&str] = &[
    r"        ╱╲     ╱╲        ",
    r"       ╱██╲   ╱██╲       ",
    r"      ╱████╲_╱████╲      ",
    r"     ┌──────────────┐    ",
    r"     │  ◕        ◕  │    ",
    r"     │      ▼▼      │    ",
    r"     │  ╱══╗██╔══╲  │    ",
    r"     │ │ ═══╝╚═══ │ │    ",
    r"     └──┬────────┬──┘    ",
    r"      ┌─┘ ░░░░░░ └─┐    ",
    r"     ┌┘  ░░░░░░░░  └┐   ",
    r"     │  ░░░░░░░░░░  │   ",
    r"     │ ░░░░░░░░░░░░ │   ",
    r"     └─┬──┘    └──┬─┘   ",
    r"  ~    └──┘    └──┘     ",
];

/// Idle frame 3: tail mid-sweep, slight inhale
pub const IDLE_3: &[&str] = &[
    r"        ╱╲     ╱╲        ",
    r"       ╱██╲   ╱██╲       ",
    r"      ╱████╲_╱████╲      ",
    r"     ┌──────────────┐    ",
    r"     │  ◕        ◕  │    ",
    r"     │      ▼▼      │    ",
    r"     │  ╱══╗██╔══╲  │    ",
    r"     │ │ ═══╝╚═══ │ │    ",
    r"     └──┬────────┬──┘    ",
    r"      ┌─┘ ░░░░░░ └─┐    ",
    r"    ┌─┘  ░░░░░░░░  └─┐  ",
    r"    │  ░░░░░░░░░░░░  │  ",
    r"    │ ░░░░░░░░░░░░░░ │  ",
    r"    └─┬──┘      └──┬─┘  ",
    r"      └──┘      └──┘~   ",
];

pub const IDLE_FRAMES: &[&[&str]] = &[IDLE_0, IDLE_1, IDLE_2, IDLE_3];

// ─────────────────────────────────────────────
//  HAPPY FRAME (eyes squint, tail up, smile)
// ─────────────────────────────────────────────

pub const HAPPY: &[&str] = &[
    r"        ╱╲     ╱╲        ",
    r"       ╱██╲   ╱██╲       ",
    r"      ╱████╲_╱████╲      ",
    r"     ┌──────────────┐    ",
    r"     │  ≧        ≦  │    ", // squinty happy eyes
    r"     │      ▼▼      │    ",
    r"     │  ╱══╗▽▽╔══╲  │    ", // open smile
    r"     │ │ ═══╝╚═══ │ │    ",
    r"     └──┬────────┬──┘    ",
    r"      ┌─┘ ░░░░░░ └─┐    ",
    r"     ┌┘  ░░░░░░░░  └┐   ",
    r"     │  ░░░░░░░░░░  │   ",
    r"     │ ░░░░░░░░░░░░ │   ",
    r"     └─┬──┘    └──┬─┘   ",
    r"       └──┘    └──┘ ⌠   ", // tail up!
];

// ─────────────────────────────────────────────
//  ANGRY FRAME (narrow eyes, arched back, tail puffed)
// ─────────────────────────────────────────────

pub const ANGRY: &[&str] = &[
    r"        ╱╲     ╱╲        ",
    r"       ╱██╲   ╱██╲       ",
    r"      ╱████╲_╱████╲      ",
    r"     ┌──────────────┐    ",
    r"     │  ◣        ◢  │    ", // angry slanted eyes
    r"     │      ▼▼      │    ",
    r"     │  ╱══╗▬▬╔══╲  │    ", // flat frown
    r"     │ │ ═══╝╚═══ │ │    ",
    r"     └──┬────────┬──┘    ",
    r"      ┌─┘ ▓▓▓▓▓▓ └─┐    ", // hackles up (denser fill)
    r"    ┌─┘  ▓▓▓▓▓▓▓▓  └─┐  ",
    r"    │  ▓▓▓▓▓▓▓▓▓▓▓▓  │  ",
    r"    │ ▓▓▓▓▓▓▓▓▓▓▓▓▓▓ │  ",
    r"    └─┬──┘      └──┬─┘  ",
    r"      └──┘      └──┘ ⌿  ", // puffed tail
];

// ─────────────────────────────────────────────
//  EATING FRAME (head down, open mouth)
// ─────────────────────────────────────────────

pub const EATING: &[&str] = &[
    r"        ╱╲     ╱╲        ",
    r"       ╱██╲   ╱██╲       ",
    r"      ╱████╲_╱████╲      ",
    r"     ┌──────────────┐    ",
    r"     │  ◕        ◕  │    ",
    r"     │      ▼▼      │    ",
    r"     │  ╱══╗  ╔══╲  │    ", // mouth open
    r"     │ │ ══╝◡◡╚══ │ │    ", // nom nom
    r"     └──┬────────┬──┘    ",
    r"      ┌─┘ ░░░░░░ └─┐    ",
    r"     ┌┘  ░░░░░░░░  └┐   ",
    r"     │  ░░░░░░░░░░  │   ",
    r"     │ ░░░░░░░░░░░░ │   ",
    r"     └─┬──┘    └──┬─┘   ",
    r"       └──┘    └──┘  ~  ",
];

// ─────────────────────────────────────────────
//  SLEEPING FRAME (closed eyes, Zzz)
// ─────────────────────────────────────────────

pub const SLEEPING: &[&str] = &[
    r"        ╱╲     ╱╲     Zzz",
    r"       ╱██╲   ╱██╲    zz ",
    r"      ╱████╲_╱████╲   z  ",
    r"     ┌──────────────┐    ",
    r"     │  ─        ─  │    ", // closed eyes
    r"     │      ▼▼      │    ",
    r"     │  ╱══╗══╔══╲  │    ", // peaceful mouth
    r"     │ │ ═══╝╚═══ │ │    ",
    r"     └──┬────────┬──┘    ",
    r"      ┌─┘ ░░░░░░ └─┐    ",
    r"     ┌┘  ░░░░░░░░  └┐   ",
    r"     │  ░░░░░░░░░░  │   ",
    r"     │ ░░░░░░░░░░░░ │   ",
    r"     └─┬──┘    └──┬─┘   ",
    r"       └──┘    └──┘ ∿   ", // relaxed tail
];

// ─────────────────────────────────────────────
//  ACCESSORY OVERLAYS
//
//  Each overlay is the same height as the cat frames (15 rows).
//  Non-space characters replace the base frame at that position.
//  The special char '·' means "transparent — keep base".
// ─────────────────────────────────────────────

/// Tiny top hat, placed between the ears
pub const OVERLAY_HAT: &[&str] = &[
    r"         ┌───┐          ", // hat brim top
    r"        ┌┤███├┐         ", // hat crown
    r"       ─┴─────┴─        ", // hat brim
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
];

/// Bow tie, placed at the neck
pub const OVERLAY_BOW: &[&str] = &[
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"       ╔═╗▓▓╔═╗         ", // bow at neck
    r"       ╚═╩══╩═╝         ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
];

/// Round glasses on the eyes
pub const OVERLAY_GLASSES: &[&str] = &[
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"      (◕)────(◕)        ", // glasses lenses + bridge
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
];

/// Scarf wrapped around neck
pub const OVERLAY_SCARF: &[&str] = &[
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"                        ",
    r"      ╔══════════╗      ", // scarf wrap
    r"      ║▒▒▒▒▒▒▒▒▒▒║     ",
    r"      ╚══╦══╦════╝      ",
    r"         ║▒▒║           ", // scarf tail hanging
    r"         ╚══╝           ",
    r"                        ",
];

/// Composite an accessory overlay on top of a base frame.
/// Non-space characters in the overlay replace the corresponding
/// character in the base frame. Returns a new Vec<String>.
pub fn composite(base: &[&str], overlay: &[&str]) -> Vec<String> {
    let rows = base.len().max(overlay.len());
    let mut result = Vec::with_capacity(rows);

    for i in 0..rows {
        let base_line: Vec<char> = if i < base.len() {
            base[i].chars().collect()
        } else {
            Vec::new()
        };
        let overlay_line: Vec<char> = if i < overlay.len() {
            overlay[i].chars().collect()
        } else {
            Vec::new()
        };

        let max_len = base_line.len().max(overlay_line.len());
        let mut merged = String::with_capacity(max_len * 4); // unicode can be multi-byte

        for j in 0..max_len {
            let oc = overlay_line.get(j).copied().unwrap_or(' ');
            let bc = base_line.get(j).copied().unwrap_or(' ');
            if oc != ' ' {
                merged.push(oc);
            } else {
                merged.push(bc);
            }
        }

        result.push(merged);
    }

    result
}

/// Get the overlay for a given accessory.
pub fn overlay_for(accessory: &super::state::Accessory) -> &'static [&'static str] {
    match accessory {
        super::state::Accessory::Hat => OVERLAY_HAT,
        super::state::Accessory::Bow => OVERLAY_BOW,
        super::state::Accessory::Glasses => OVERLAY_GLASSES,
        super::state::Accessory::Scarf => OVERLAY_SCARF,
    }
}
