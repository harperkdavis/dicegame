pub struct FlagsDef(Vec<&'static str>);

impl FlagsDef {
    pub fn load() -> Self {
        let str = include_str!("../../../cnt/FLAGS");
        Self(str.lines().collect())
    }
}
