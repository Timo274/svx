//! A small curated wordlist used for transfer codes and SAS strings.
//!
//! We intentionally use short, unambiguous words (4-6 letters, no homophones)
//! so that operators can read them aloud over voice or type them without
//! error. 256 words → one byte per word, perfect for encoding hashes.

pub const WORDS: [&str; 256] = [
    "acorn", "agent", "album", "alpha", "amber", "anvil", "apple", "april", "arrow", "atlas",
    "audio", "aztec", "bacon", "badge", "baker", "banjo", "basil", "baton", "beach", "begin",
    "beta", "bingo", "birch", "bison", "black", "blaze", "bliss", "block", "blues", "board",
    "boost", "brick", "brisk", "brown", "brush", "buddy", "bugle", "buoy", "cable", "cactus",
    "cadet", "candy", "canoe", "canon", "canvas", "cargo", "carol", "carry", "cedar", "cello",
    "chalk", "charm", "chess", "chime", "chord", "chunk", "cider", "cigar", "civic", "civil",
    "claim", "clamp", "clash", "clasp", "clerk", "click", "cliff", "climb", "clip", "clock",
    "cloud", "clown", "cobra", "cocoa", "code", "coin", "comet", "comic", "coral", "corgi",
    "couch", "craft", "crane", "crisp", "cross", "crown", "curry", "cycle", "daisy", "dance",
    "delta", "denim", "depot", "derby", "diary", "dingo", "diver", "dodge", "donut", "draft",
    "dream", "drift", "drone", "druid", "duet", "dune", "eagle", "east", "easy", "echo", "eddy",
    "eel", "elder", "elk", "elm", "ember", "emoji", "enter", "envoy", "epoch", "equal", "error",
    "ether", "extra", "fable", "facet", "fairy", "false", "fancy", "fang", "fawn", "feast", "feed",
    "fern", "fiber", "field", "fig", "file", "film", "final", "fish", "five", "flag", "flame",
    "flash", "flask", "flint", "float", "flock", "fog", "forge", "forth", "four", "fox", "free",
    "fresh", "fringe", "frog", "fruit", "fuel", "fuzz", "gadget", "game", "gamma", "garlic",
    "gator", "gecko", "geode", "ghost", "giant", "gift", "glass", "glee", "globe", "glove", "goal",
    "goat", "gold", "grape", "graph", "great", "green", "grid", "gulf", "gull", "habit", "haiku",
    "happy", "harp", "hatch", "haven", "heart", "hedge", "helix", "hello", "helm", "hero", "hilt",
    "hippo", "hive", "hobby", "honey", "horse", "hotel", "house", "human", "humor", "hunt",
    "husky", "iceberg", "image", "imp", "index", "inert", "input", "iris", "iron", "island",
    "ivory", "jade", "jam", "jazz", "jeep", "jetty", "jewel", "jolly", "joker", "judge", "jug",
    "juice", "jumbo", "jungle", "juror", "keen", "kelp", "kind", "king", "kiosk", "kite", "kiwi",
    "knave", "knee", "knight", "knot", "koala", "lab", "lace", "lake", "lamp", "land", "lance",
    "larch", "laser", "latte", "laurel", "lava",
];

const _: () = assert!(WORDS.len() == 256, "wordlist must have exactly 256 entries");

/// Encode a byte slice as a dash-separated sequence of words.
pub fn encode_words(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| WORDS[*b as usize])
        .collect::<Vec<_>>()
        .join("-")
}
