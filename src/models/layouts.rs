use super::{ClassicLedConfig, Led, Leds};

/// Trait for converting a LED configuration to LEDs
pub trait ToLeds {
    fn to_leds(&self) -> Leds;
}

struct ClassicLedParams {
    ledstop: u32,
    ledsbottom: u32,
    ledsleft: u32,
    ledsright: u32,
    ledsglength: u32,
    ledsgpos: u32,
    position: i32,
    reverse: bool,
    ledsvdepth: f32,
    ledshdepth: f32,
    edgehgap: f32,
    edgevgap: f32,
    overlap: f32,
    ptblh: f32,
    ptblv: f32,
    ptbrh: f32,
    ptbrv: f32,
    pttlh: f32,
    pttlv: f32,
    pttrh: f32,
    pttrv: f32,
}

impl From<&ClassicLedConfig> for ClassicLedParams {
    fn from(c: &ClassicLedConfig) -> Self {
        Self {
            ledstop: c.top,
            ledsbottom: c.bottom,
            ledsleft: c.left,
            ledsright: c.right,
            ledsglength: c.glength,
            ledsgpos: c.gpos,
            position: c.position,
            reverse: c.reverse,
            ledsvdepth: c.vdepth as f32 / 100.,
            ledshdepth: c.hdepth as f32 / 100.,
            edgehgap: (c.edgegap as f32 / 200.) / (16. / 9.),
            edgevgap: c.edgegap as f32 / 200.,
            overlap: c.overlap as f32 / 100.,
            ptblh: c.pblh as f32 / 100.,
            ptblv: c.pblv as f32 / 100.,
            ptbrh: c.pbrh as f32 / 100.,
            ptbrv: c.pbrv as f32 / 100.,
            pttlh: c.ptlh as f32 / 100.,
            pttlv: c.ptlv as f32 / 100.,
            pttrh: c.ptrh as f32 / 100.,
            pttrv: c.ptrv as f32 / 100.,
        }
    }
}

impl ClassicLedParams {
    fn round(x: f32) -> f32 {
        let factor = 1e4;
        (x * factor).round() / factor
    }

    fn create_led(hmin: f32, hmax: f32, vmin: f32, vmax: f32) -> Led {
        Led {
            hmin: Self::round(hmin),
            hmax: Self::round(hmax),
            vmin: Self::round(vmin),
            vmax: Self::round(vmax),
            color_order: None,
            name: None,
        }
    }

    fn ovl_plus(&self, x: f32) -> f32 {
        (x + self.overlap).clamp(0., 1.)
    }

    fn ovl_minus(&self, x: f32) -> f32 {
        (x - self.overlap).clamp(0., 1.)
    }

    fn create_top_leds(&self, leds: &mut Vec<Led>) {
        let steph = (self.pttrh - self.pttlh - (2. * self.edgehgap)) as f32 / self.ledstop as f32;
        let stepv = (self.pttrv - self.pttlv) as f32 / self.ledstop as f32;

        leds.reserve(self.ledstop as _);
        for i in 0..self.ledstop {
            let i = i as f32;
            let hmin = self.ovl_minus(self.pttlh + (steph * i) + self.edgehgap);
            let hmax = self.ovl_plus(self.pttlh + (steph * (i + 1.)) + self.edgehgap);
            let vmin = self.pttlv + (stepv * i);
            let vmax = vmin + self.ledshdepth;

            leds.push(Self::create_led(hmin, hmax, vmin, vmax));
        }
    }

    fn create_right_leds(&self, leds: &mut Vec<Led>) {
        let steph = (self.ptbrh - self.pttrh) as f32 / self.ledsright as f32;
        let stepv = (self.ptbrv - self.pttrv - (2. * self.edgevgap)) / self.ledsright as f32;

        leds.reserve(self.ledsright as _);
        for i in 0..self.ledsright {
            let i = i as f32;
            let hmax = self.pttrh + (steph * (i + 1.));
            let hmin = hmax - self.ledsvdepth;
            let vmin = self.ovl_minus(self.pttrv + (stepv * i) + self.edgevgap);
            let vmax = self.ovl_plus(self.pttrv + (stepv * (i + 1.)) + self.edgevgap);

            leds.push(Self::create_led(hmin, hmax, vmin, vmax));
        }
    }

    fn create_bottom_leds(&self, leds: &mut Vec<Led>) {
        let steph =
            (self.ptbrh - self.ptblh - (2. * self.edgehgap)) as f32 / self.ledsbottom as f32;
        let stepv = (self.ptbrv - self.ptblv) as f32 / self.ledsbottom as f32;

        leds.reserve(self.ledsbottom as _);
        for i in 0..self.ledsbottom {
            let i = i as f32;
            let hmin = self.ovl_minus(self.ptblh + (steph * i) + self.edgehgap);
            let hmax = self.ovl_plus(self.ptblh + (steph * (i + 1.)) + self.edgehgap);
            let vmax = self.ptblv + (stepv * i);
            let vmin = vmax - self.ledshdepth;

            leds.push(Self::create_led(hmin, hmax, vmin, vmax));
        }
    }

    fn create_left_leds(&self, leds: &mut Vec<Led>) {
        let steph = (self.ptblh - self.pttlh) as f32 / self.ledsleft as f32;
        let stepv = (self.ptblv - self.pttlv - (2. * self.edgevgap)) as f32 / self.ledsleft as f32;

        leds.reserve(self.ledsleft as _);
        for i in 0..self.ledsleft {
            let i = i as f32;
            let hmin = self.pttlh + (steph * i);
            let hmax = hmin + self.ledsvdepth;
            let vmin = self.ovl_minus(self.pttlv + (stepv * i) + self.edgevgap);
            let vmax = self.ovl_plus(self.pttlv + (stepv * (i + 1.)) + self.edgevgap);

            leds.push(Self::create_led(hmin, hmax, vmin, vmax));
        }
    }
}

impl ToLeds for ClassicLedParams {
    fn to_leds(&self) -> Leds {
        let mut leds = Vec::with_capacity(
            (self.ledstop + self.ledsbottom + self.ledsleft + self.ledsright) as usize,
        );

        self.create_top_leds(&mut leds);
        self.create_right_leds(&mut leds);
        self.create_bottom_leds(&mut leds);
        self.create_left_leds(&mut leds);

        // Check LED gap pos
        let ledsgpos = if self.ledsgpos + self.ledsglength > leds.len() as _ {
            (leds.len() as isize - self.ledsglength as isize).max(0) as usize
        } else {
            self.ledsglength as usize
        };

        // Check LED gap length
        let ledsglength = if self.ledsglength >= leds.len() as _ {
            leds.len() as isize - self.ledsglength as isize - 1
        } else {
            self.ledsglength as _
        };

        if ledsglength > 0 {
            leds.splice(
                ledsgpos..(ledsgpos + ledsglength as usize),
                std::iter::empty(),
            );
        }

        if self.position < 0 {
            leds.rotate_left(-self.position as _);
        } else if self.position > 0 {
            leds.rotate_right(self.position as _);
        }

        if self.reverse {
            leds.reverse();
        }

        Leds { leds }
    }
}

impl ToLeds for ClassicLedConfig {
    fn to_leds(&self) -> Leds {
        ClassicLedParams::from(self).to_leds()
    }
}
