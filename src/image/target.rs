use anyhow::Result;

pub(super) struct Target {
    width: u32,
    height: u32,
}

impl Target {
    pub(super) fn new(width: u32, height: u32) -> Result<Self> {
        if width == 0 || height == 0 {
            anyhow::bail!("render target dimensions must be non-zero");
        }

        Ok(Self { width, height })
    }

    pub(super) fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    pub(super) fn width(&self) -> u32 {
        self.width
    }

    pub(super) fn height(&self) -> u32 {
        self.height
    }
}
