use arcstr::ArcStr;

pub struct Error {
    level: Level,
    message: ArcStr,
}

enum Level {
    Error,
    Warn,
    Info,
    Hint,
}
