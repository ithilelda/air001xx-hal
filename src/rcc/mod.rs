const HSI_CLK_RATE: [u32;5] = [4_000_000, 8_000_000, 16_000_000, 22_120_000, 24_000_000];
const HSI_TRIM_ADDR: [u32;5] = [0x1FFF_0F00, 0x1FFF_0F04, 0x1FFF_0F08, 0x1FFF_0F0C, 0x1FFF_0F10];

#[derive(Copy, Clone)]
pub enum HsiClockRate {
    MHz4 = 0b000,
    MHz8 = 0b001,
    MHz16 = 0b010,
    MHz22 = 0b011,
    MHz24 = 0b100,
}
impl HsiClockRate {
    pub const fn clk_rate(self) -> u32 {
        HSI_CLK_RATE[self as usize]
    }
    pub const fn trim_addr(self) -> u32 {
        HSI_TRIM_ADDR[self as usize]
    }
    pub fn trim_value(self) -> u16 {
        unsafe { core::ptr::read_volatile(self.trim_addr() as *const u16) & 0x1FFF }
    }
}

#[derive(Copy, Clone)]
pub enum HsiDiv {
    Div1 = 0b000,
    Div2 = 0b001,
    Div4 = 0b010,
    Div8 = 0b011,
    Div16 = 0b100,
    Div32 = 0b101,
    Div64 = 0b110,
    Div128 = 0b111,
}

#[derive(Copy, Clone)]
pub struct HsiConfig {
    pub rate: HsiClockRate,
    pub div: HsiDiv,
}

#[derive(Copy, Clone)]
pub enum SysClkSource {
    Hsi = 0b000,
    Hse = 0b001,
    Pll = 0b010,
    Lsi = 0b011,
    Lse = 0b100,
}

#[derive(Copy, Clone)]
pub enum PllSource {
    Hsi = 0b0,
    Hse = 0b1,
}

pub struct SysClkConfig {
    pub clock_source: SysClkSource,
    pub pll_source: Option<PllSource>,
    pub hsi_config: Option<HsiConfig>,
}

pub fn configure_hsi(config: &HsiConfig, rcc: &air001xx_pac::Rcc) -> u32 {
    rcc.cr().modify(|_, w| unsafe {
        w.hsion().set_bit()
         .hsidiv().bits(config.div as u8)
    }); // enable HSI and set div.
    rcc.icscr().modify(|_, w| unsafe {
        w.hsi_fs().bits(config.rate as u8)
         .hsi_trim().bits(config.rate.trim_value())
    }); // set HSI frequency and read corresponding trim value.
    while rcc.cr().read().hsirdy().bit_is_clear() {
        // wait for HSI ready.
    }
    config.rate.clk_rate() >> config.div as u8
}

pub fn configure_sys_clk(config: &SysClkConfig, rcc: &air001xx_pac::Rcc, flash: &air001xx_pac::Flash) -> u32 {
    let sys_clk = match config.clock_source {
        SysClkSource::Hsi => {
            let hsi_config = config.hsi_config.unwrap(); // explicitly copy hsi_config and panic if None.
            configure_hsi(&hsi_config, rcc)
        },
        SysClkSource::Hse => {
            todo!();
        },
        SysClkSource::Pll => {
            match config.pll_source {
                Some(PllSource::Hsi) => {
                    let hsi_config = config.hsi_config.unwrap();
                    let hsi_clk = configure_hsi(&hsi_config, rcc);
                    rcc.pllcfgr().modify(|_, w| w.pllsrc().clear_bit()); // set HSI as PLL source.
                    rcc.cr().modify(|_, w| w.pllon().set_bit()); // enable PLL.
                    hsi_clk * 2
                },
                Some(PllSource::Hse) => {
                    todo!();
                },
                None => panic!(),
            }
        }
        SysClkSource::Lsi => {
            todo!();
        }
        SysClkSource::Lse => {
            todo!();
        }
    };
    if sys_clk > 24_000_000 {
        flash.acr().write(|w| w.latency().set_bit()); // set flash latency to 1 above 24MHz sys clk.
    }
    rcc.cfgr().modify(|_, w| unsafe { w.sw().bits(config.clock_source as u8) }); // set sys clk source.
    while rcc.cfgr().read().sws().bits() != config.clock_source as u8 {
        // wait for the desinated source selected as system clock source.
    }
    sys_clk
}