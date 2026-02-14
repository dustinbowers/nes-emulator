pub struct DmcDma {}

impl DmcDma {
    pub fn new() -> DmcDma {
        DmcDma {}
    }

    pub fn wants_bus(&self) -> bool {
        false
    }
}
