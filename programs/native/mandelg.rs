//! MANDELG, written directly in Rust (not translated from the DOS binary).
//!
//! Same behaviour as programs/dos/MANDELG.PAS: 320x200 Mandelbrot, coarse 3x3
//! pass first, mouse zoom (left in, right out) with a preview of the old
//! frame, HUD line "Z<level> I<iterations> T:<time> (<cx>,<cy>)", `R` resets,
//! `Q`/`Esc` quits. Windows only; no external crates.
//!
//! Build: rustc --edition=2021 -C opt-level=3 -C strip=symbols -C panic=abort \
//!        -C lto=fat -C codegen-units=1 programs/native/mandelg.rs -o <out>
#![cfg_attr(windows, windows_subsystem = "windows")]

const WIDTH: usize = 320;
const HEIGHT: usize = 200;
#[cfg(windows)]
const HUD_ROWS: usize = 9;
const MAX_ITER: i32 = 50_000;

#[derive(Clone, Copy, PartialEq)]
enum Event {
    ZoomIn(i32, i32),
    ZoomOut(i32, i32),
    Reset,
    Quit,
}

/// A finished view kept so zoom out can bring the wider region back unchanged.
struct Saved {
    frame: Vec<u8>,
    cx: f64,
    cy: f64,
    span_x: f64,
    span_y: f64,
    zoom: i32,
    max_iter: i32,
    dx: f64,
    dy: f64,
    complete: bool,
}

struct App {
    history: Vec<Saved>,
    complete: bool,
    frame: Vec<u8>,
    temp: Vec<u8>,
    cx: f64,
    cy: f64,
    span_x: f64,
    span_y: f64,
    zoom: i32,
    max_iter: i32,
    frame_cs: i64,
    xmin: f64,
    ymin: f64,
    dx: f64,
    dy: f64,
    has_box: bool,
    bx0: i32,
    bx1: i32,
    by0: i32,
    by1: i32,
    pending: Option<Event>,
    last_present: std::time::Instant,
}

#[cfg(windows)]
fn palette() -> [u32; 256] {
    let mut p = [0u32; 256];
    for i in 1..256i32 {
        let (r, g, b) = if i < 64 {
            (0, i / 2, i)
        } else if i < 128 {
            (i - 64, 31 + (i - 64) / 2, 63)
        } else if i < 192 {
            (63, 63 - (i - 128) / 2, 63 - (i - 128))
        } else {
            (63 - (i - 192), 31 - (i - 192) / 2, 0)
        };
        p[i as usize] = rgb6(r, g, b);
    }
    p[255] = rgb6(63, 63, 63);
    p
}
#[cfg(windows)]
fn rgb6(r: i32, g: i32, b: i32) -> u32 {
    let s = |v: i32| (v * 255 / 63) as u32;
    0xff00_0000 | s(r) << 16 | s(g) << 8 | s(b)
}

/// Each 3x zoom doubles the limit, so deep, nearly self-similar
/// regions keep resolving instead of filling in black (was +32 per level, max 512); now capped at MAX_ITER.
fn iterations_for(zoom: i32) -> i32 {
    (32.0 * 2.0f64.powi(zoom.max(0))).min(MAX_ITER as f64) as i32
}

fn fmt_fixed(mut v: f64, decimals: usize) -> String {
    let mut s = String::new();
    if v < 0.0 {
        s.push('-');
        v = -v;
    }
    if v >= 10.0 {
        s.push_str(">9");
        return s;
    }
    let digit = |v: &mut f64| {
        let mut d = 0;
        while d < 9 && *v >= 1.0 {
            *v -= 1.0;
            d += 1;
        }
        (b'0' + d) as char
    };
    s.push(digit(&mut v));
    s.push('.');
    for _ in 0..decimals {
        v *= 10.0;
        s.push(digit(&mut v));
    }
    s
}

fn fmt_time_cs(cs: i64) -> String {
    if cs < 0 {
        "...".into()
    } else {
        format!("{}.{:02}s", cs / 100, cs % 100)
    }
}

fn calc_color(px: f64, py: f64, max_iter: i32) -> u8 {
    let (mut x, mut y) = (0.0f64, 0.0f64);
    let mut iter = 0;
    while iter < max_iter {
        let (xx, yy) = (x * x, y * y);
        if xx + yy > 4.0 {
            break;
        }
        let nx = xx - yy + px;
        y = 2.0 * x * y + py;
        x = nx;
        iter += 1;
    }
    if iter == max_iter {
        0
    } else {
        (1 + ((iter * 12 + 24) % 254)) as u8
    }
}

impl App {
    fn new() -> Self {
        let mut a = App {
            history: Vec::new(),
            complete: false,
            frame: vec![0; WIDTH * HEIGHT],
            temp: vec![0; WIDTH * HEIGHT],
            cx: 0.0,
            cy: 0.0,
            span_x: 0.0,
            span_y: 0.0,
            zoom: 0,
            max_iter: 0,
            frame_cs: -1,
            xmin: 0.0,
            ymin: 0.0,
            dx: 0.0,
            dy: 0.0,
            has_box: false,
            bx0: WIDTH as i32,
            bx1: -1,
            by0: HEIGHT as i32,
            by1: -1,
            pending: None,
            last_present: std::time::Instant::now(),
        };
        a.reset_view();
        a
    }
    fn reset_view(&mut self) {
        self.cx = -0.6;
        self.cy = 0.0;
        self.span_x = 3.2;
        self.span_y = 2.4;
        self.zoom = 0;
        self.max_iter = iterations_for(0);
    }
    fn hud(&self) -> String {
        format!(
            "Z{} I{} T:{} ({},{})",
            self.zoom,
            self.max_iter,
            fmt_time_cs(self.frame_cs),
            fmt_fixed(self.cx, 3),
            fmt_fixed(self.cy, 3)
        )
    }
    fn in_box(&self, c: i32, r: i32) -> bool {
        self.has_box && c >= self.bx0 && c <= self.bx1 && r >= self.by0 && r <= self.by1
    }
    fn present(&mut self) {
        self.last_present = std::time::Instant::now();
        let hud = self.hud();
        host::present(&self.frame, &hud);
    }
    /// Gather input; true when an action was queued (Pascal PollInput).
    fn poll(&mut self) -> bool {
        host::pump();
        if self.last_present.elapsed().as_millis() >= 30 {
            self.present();
        }
        while let Some(ev) = host::next_event() {
            if matches!(ev, Event::ZoomOut(..)) && self.span_x >= 25.0 {
                continue;
            }
            self.pending = Some(ev);
            return true;
        }
        false
    }

    /// Zoom out shrinks the old frame 3x: each new pixel stands for a 3x3 block
    /// of old pixels around (x, y). Use the most frequent colour of those nine
    /// (centre wins ties) instead of recalculating it.
    fn block_color(&self, x: i32, y: i32) -> u8 {
        let mut cells = [0u8; 9];
        let mut n = 0;
        for (dy, dx) in [(0, 0), (-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)] {
            let (px, py) = (x + dx, y + dy);
            if px >= 0 && px < WIDTH as i32 && py >= 0 && py < HEIGHT as i32 {
                cells[n] = self.frame[py as usize * WIDTH + px as usize];
                n += 1;
            }
        }
        let (mut best, mut best_count) = (cells[0], 0);
        for &c in &cells[..n] {
            let count = cells[..n].iter().filter(|&&o| o == c).count();
            if count > best_count {
                (best, best_count) = (c, count);
            }
        }
        best
    }

    fn preview_zoom(&mut self, zoom_in: bool, click_x: i32, click_y: i32) {
        self.has_box = false;
        self.bx0 = WIDTH as i32;
        self.bx1 = -1;
        self.by0 = HEIGHT as i32;
        self.by1 = -1;
        for r in 0..HEIGHT as i32 {
            let sy = if zoom_in {
                click_y - 33 + r / 3
            } else {
                click_y + (r - 100) * 3
            };
            let row = r as usize * WIDTH;
            if sy < 0 || sy >= HEIGHT as i32 {
                self.temp[row..row + WIDTH].fill(0);
                continue;
            }
            for c in 0..WIDTH as i32 {
                let sx = if zoom_in {
                    click_x - 53 + c / 3
                } else {
                    click_x + (c - 160) * 3
                };
                if sx >= 0 && sx < WIDTH as i32 {
                    self.temp[row + c as usize] = if zoom_in {
                        self.frame[sy as usize * WIDTH + sx as usize]
                    } else {
                        self.block_color(sx, sy)
                    };
                    if !zoom_in {
                        if !self.has_box {
                            self.has_box = true;
                            (self.bx0, self.bx1, self.by0, self.by1) = (c, c, r, r);
                        } else {
                            self.bx0 = self.bx0.min(c);
                            self.bx1 = self.bx1.max(c);
                            self.by0 = self.by0.min(r);
                            self.by1 = self.by1.max(r);
                        }
                    }
                } else {
                    self.temp[row + c as usize] = 0;
                }
            }
        }
        std::mem::swap(&mut self.frame, &mut self.temp);
        self.present();
    }

    fn render(&mut self, coarse_first: bool) {
        let (w, h) = (WIDTH as i32, HEIGHT as i32);
        self.xmin = self.cx - self.span_x * 0.5;
        let xmax = self.cx + self.span_x * 0.5;
        self.ymin = self.cy - self.span_y * 0.5;
        let ymax = self.cy + self.span_y * 0.5;
        self.dx = (xmax - self.xmin) / WIDTH as f64;
        self.dy = (ymax - self.ymin) / HEIGHT as f64;

        self.frame_cs = -1;
        self.complete = false;
        self.present();
        let t0 = std::time::Instant::now();
        self.pending = None;
        let mut interrupted = false;

        if coarse_first {
            let mut br = 0;
            'rows: while br < h {
                let cy = self.ymin + br as f64 * self.dy;
                let rmax = (br + 2).min(h - 1);
                let mut bc = 0;
                while bc < w {
                    if !self.in_box(bc, br) {
                        let cx = self.xmin + bc as f64 * self.dx;
                        let color = calc_color(cx, cy, self.max_iter);
                        let cmax = (bc + 2).min(w - 1);
                        for r2 in br..=rmax {
                            for c2 in bc..=cmax {
                                if !self.in_box(c2, r2) {
                                    self.frame[r2 as usize * WIDTH + c2 as usize] = color;
                                }
                            }
                        }
                    }
                    if self.max_iter >= 64 && (bc & 31) == 0 && self.poll() {
                        interrupted = true;
                        break 'rows;
                    }
                    bc += 3;
                }
                if self.poll() {
                    interrupted = true;
                    break;
                }
                br += 3;
            }
        }

        if !interrupted {
            'fine: for row in 0..h {
                let cy = self.ymin + row as f64 * self.dy;
                for col in 0..w {
                    // Pixels carried over from the previous frame (zoom-out box) are kept.
                    let skip = self.in_box(col, row) || (coarse_first && row % 3 == 0 && col % 3 == 0);
                    if !skip {
                        let cx = self.xmin + col as f64 * self.dx;
                        self.frame[row as usize * WIDTH + col as usize] =
                            calc_color(cx, cy, self.max_iter);
                    }
                    if self.max_iter >= 64 && (col & 31) == 31 && self.poll() {
                        interrupted = true;
                        break 'fine;
                    }
                }
                if self.poll() {
                    interrupted = true;
                    break;
                }
            }
        }

        if !interrupted {
            self.frame_cs = (t0.elapsed().as_millis() / 10) as i64;
            self.has_box = false;
            self.complete = true;
        }
        self.present();
    }

    fn run(&mut self) {
        self.frame.fill(0);
        self.has_box = false;
        self.render(true);
        loop {
            if self.pending.is_none() {
                self.poll();
            }
            let Some(ev) = self.pending.take() else {
                host::wait();
                continue;
            };
            match ev {
                Event::ZoomIn(x, y) => {
                    self.history.push(Saved {
                        frame: self.frame.clone(),
                        cx: self.cx,
                        cy: self.cy,
                        span_x: self.span_x,
                        span_y: self.span_y,
                        zoom: self.zoom,
                        max_iter: self.max_iter,
                        dx: self.dx,
                        dy: self.dy,
                        complete: self.complete,
                    });
                    self.preview_zoom(true, x, y);
                    self.cx = self.xmin + x as f64 * self.dx;
                    self.cy = self.ymin + y as f64 * self.dy;
                    self.span_x /= 3.0;
                    self.span_y /= 3.0;
                    self.zoom += 1;
                    self.max_iter = iterations_for(self.zoom);
                    self.render(false);
                }
                Event::ZoomOut(x, y) if !self.history.is_empty() => {
                    // The wider view we zoomed in from has the scale of the new
                    // view, so its pixels are reused, shifted so the region is
                    // centred on the click. Only the uncovered strip is calculated.
                    let v = self.history.pop().unwrap();
                    let want_x = self.xmin + x as f64 * self.dx;
                    let want_y = self.ymin + y as f64 * self.dy;
                    let shx = ((want_x - v.cx) / v.dx).round() as i32;
                    let shy = ((want_y - v.cy) / v.dy).round() as i32;
                    // Snap the centre to the saved pixel grid so shifted pixels are exact.
                    self.cx = v.cx + shx as f64 * v.dx;
                    self.cy = v.cy + shy as f64 * v.dy;
                    (self.span_x, self.span_y) = (v.span_x, v.span_y);
                    (self.zoom, self.max_iter) = (v.zoom, v.max_iter);
                    self.has_box = false;
                    if v.complete {
                        let (w, h) = (WIDTH as i32, HEIGHT as i32);
                        self.temp.fill(0);
                        let (c0, c1) = ((-shx).max(0), (w - shx).min(w) - 1);
                        let (r0, r1) = ((-shy).max(0), (h - shy).min(h) - 1);
                        if c0 <= c1 && r0 <= r1 {
                            for r in r0..=r1 {
                                for c in c0..=c1 {
                                    self.temp[r as usize * WIDTH + c as usize] =
                                        v.frame[(r + shy) as usize * WIDTH + (c + shx) as usize];
                                }
                            }
                            self.has_box = true;
                            (self.bx0, self.bx1, self.by0, self.by1) = (c0, c1, r0, r1);
                        }
                        std::mem::swap(&mut self.frame, &mut self.temp);
                    } else {
                        self.frame.fill(0);
                    }
                    self.render(true);
                }
                Event::ZoomOut(x, y) => {
                    if self.span_x < 25.0 {
                        self.preview_zoom(false, x, y);
                        self.cx = self.xmin + x as f64 * self.dx;
                        self.cy = self.ymin + y as f64 * self.dy;
                        self.span_x *= 3.0;
                        self.span_y *= 3.0;
                        self.zoom -= 1;
                        self.max_iter = iterations_for(self.zoom);
                        self.render(true);
                    }
                }
                Event::Reset => {
                    self.reset_view();
                    self.history.clear();
                    self.has_box = false;
                    self.frame.fill(0);
                    self.render(true);
                }
                Event::Quit => break,
            }
        }
    }
}

#[cfg(windows)]
mod host {
    use super::{palette, Event, HEIGHT, HUD_ROWS, WIDTH};
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicIsize, Ordering};
    use std::sync::Mutex;

    static HWND: AtomicIsize = AtomicIsize::new(0);
    static EVENTS: Mutex<VecDeque<Event>> = Mutex::new(VecDeque::new());
    static PIXELS: Mutex<(Vec<u32>, String)> = Mutex::new((Vec::new(), String::new()));

    #[repr(C)]
    struct Msg {
        hwnd: isize,
        message: u32,
        wparam: usize,
        lparam: isize,
        time: u32,
        pt: [i32; 2],
        private: u32,
    }
    #[repr(C)]
    struct WndClass {
        style: u32,
        proc_: unsafe extern "system" fn(isize, u32, usize, isize) -> isize,
        cls_extra: i32,
        wnd_extra: i32,
        instance: isize,
        icon: isize,
        cursor: isize,
        background: isize,
        menu: *const u16,
        class_name: *const u16,
    }
    #[repr(C)]
    struct PaintStruct {
        hdc: isize,
        erase: i32,
        rc: [i32; 4],
        restore: i32,
        update: i32,
        reserved: [u8; 32],
    }
    #[repr(C)]
    struct BitmapInfo {
        size: u32,
        width: i32,
        height: i32,
        planes: u16,
        bits: u16,
        compression: u32,
        image: u32,
        xppm: i32,
        yppm: i32,
        used: u32,
        important: u32,
    }

    #[link(name = "user32")]
    extern "system" {
        fn RegisterClassW(c: *const WndClass) -> u16;
        fn CreateWindowExW(ex: u32, class: *const u16, title: *const u16, style: u32, x: i32, y: i32, w: i32, h: i32, parent: isize, menu: isize, inst: isize, param: isize) -> isize;
        fn AdjustWindowRect(rc: *mut [i32; 4], style: u32, menu: i32) -> i32;
        fn ShowWindow(h: isize, cmd: i32) -> i32;
        fn DestroyWindow(h: isize) -> i32;
        fn PeekMessageW(m: *mut Msg, h: isize, min: u32, max: u32, remove: u32) -> i32;
        fn TranslateMessage(m: *const Msg) -> i32;
        fn DispatchMessageW(m: *const Msg) -> isize;
        fn DefWindowProcW(h: isize, m: u32, w: usize, l: isize) -> isize;
        fn WaitMessage() -> i32;
        fn GetDC(h: isize) -> isize;
        fn ReleaseDC(h: isize, dc: isize) -> i32;
        fn BeginPaint(h: isize, ps: *mut PaintStruct) -> isize;
        fn EndPaint(h: isize, ps: *const PaintStruct) -> i32;
        fn GetClientRect(h: isize, rc: *mut [i32; 4]) -> i32;
        fn LoadCursorW(inst: isize, name: usize) -> isize;
    }
    #[link(name = "gdi32")]
    extern "system" {
        fn StretchDIBits(dc: isize, xd: i32, yd: i32, wd: i32, hd: i32, xs: i32, ys: i32, ws: i32, hs: i32, bits: *const u32, bmi: *const BitmapInfo, usage: u32, rop: u32) -> i32;
        fn SetTextColor(dc: isize, c: u32) -> u32;
        fn SetBkMode(dc: isize, mode: i32) -> i32;
        fn CreateFontW(h: i32, w: i32, e: i32, o: i32, weight: i32, i: u32, u: u32, s: u32, cs: u32, op: u32, cp: u32, q: u32, pf: u32, face: *const u16) -> isize;
        fn SelectObject(dc: isize, o: isize) -> isize;
        fn DeleteObject(o: isize) -> i32;
        fn TextOutA(dc: isize, x: i32, y: i32, s: *const u8, n: i32) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleW(name: *const u16) -> isize;
    }

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }
    fn client(h: isize) -> (i32, i32) {
        let mut rc = [0i32; 4];
        unsafe { GetClientRect(h, &mut rc) };
        ((rc[2] - rc[0]).max(1), (rc[3] - rc[1]).max(1))
    }

    fn paint(h: isize, dc: isize) {
        let (cw, ch) = client(h);
        let guard = PIXELS.lock().unwrap();
        let bmi = BitmapInfo {
            size: 40,
            width: WIDTH as i32,
            height: -(HEIGHT as i32),
            planes: 1,
            bits: 32,
            compression: 0,
            image: 0,
            xppm: 0,
            yppm: 0,
            used: 0,
            important: 0,
        };
        unsafe {
            StretchDIBits(dc, 0, 0, cw, ch, 0, 0, WIDTH as i32, HEIGHT as i32, guard.0.as_ptr(), &bmi, 0, 0x00CC_0020);
            let face = wide("Courier New");
            let font = CreateFontW(-(ch * 8 / HEIGHT as i32).max(8), 0, 0, 0, 700, 0, 0, 0, 0, 0, 0, 0, 49, face.as_ptr());
            let old = SelectObject(dc, font);
            SetBkMode(dc, 1);
            SetTextColor(dc, 0x00ff_ffff);
            TextOutA(dc, 0, 0, guard.1.as_ptr(), guard.1.len() as i32);
            SelectObject(dc, old);
            DeleteObject(font);
        }
    }

    pub fn present(frame: &[u8], hud: &str) {
        {
            let pal = palette();
            let mut g = PIXELS.lock().unwrap();
            for (i, p) in g.0.iter_mut().enumerate() {
                *p = if i < HUD_ROWS * WIDTH { 0xff00_0000 } else { pal[frame[i] as usize] };
            }
            g.1 = hud.to_string();
        }
        let h = HWND.load(Ordering::Relaxed);
        unsafe {
            let dc = GetDC(h);
            paint(h, dc);
            ReleaseDC(h, dc);
        }
    }

    unsafe extern "system" fn wndproc(h: isize, msg: u32, w: usize, l: isize) -> isize {
        let mouse = |out: bool| {
            let (cw, ch) = client(h);
            let x = (l as i16 as i32 * WIDTH as i32 / cw).clamp(0, WIDTH as i32 - 1);
            let y = ((l >> 16) as i16 as i32 * HEIGHT as i32 / ch).clamp(0, HEIGHT as i32 - 1);
            EVENTS.lock().unwrap().push_back(if out { Event::ZoomOut(x, y) } else { Event::ZoomIn(x, y) });
        };
        match msg {
            0x0201 => mouse(false),
            0x0204 => mouse(true),
            0x0102 => match w as u8 {
                27 | b'q' | b'Q' => EVENTS.lock().unwrap().push_back(Event::Quit),
                b'r' | b'R' => EVENTS.lock().unwrap().push_back(Event::Reset),
                _ => {}
            },
            0x0010 => EVENTS.lock().unwrap().push_back(Event::Quit),
            0x0014 => return 1,
            0x000F => {
                let mut ps: PaintStruct = unsafe { std::mem::zeroed() };
                let dc = BeginPaint(h, &mut ps);
                paint(h, dc);
                EndPaint(h, &ps);
                return 0;
            }
            _ => {}
        }
        DefWindowProcW(h, msg, w, l)
    }

    pub fn init() {
        PIXELS.lock().unwrap().0 = vec![0xff00_0000; WIDTH * HEIGHT];
        unsafe {
            let inst = GetModuleHandleW(std::ptr::null());
            let class = wide("PilMandelgRust");
            let wc = WndClass {
                style: 0,
                proc_: wndproc,
                cls_extra: 0,
                wnd_extra: 0,
                instance: inst,
                icon: 0,
                cursor: LoadCursorW(0, 32512),
                background: 0,
                menu: std::ptr::null(),
                class_name: class.as_ptr(),
            };
            RegisterClassW(&wc);
            let style = 0x00CF_0000; // WS_OVERLAPPEDWINDOW
            let mut rc = [0, 0, WIDTH as i32 * 3, HEIGHT as i32 * 3];
            AdjustWindowRect(&mut rc, style, 0);
            let title = wide("MANDELG (Rust)");
            let h = CreateWindowExW(0, class.as_ptr(), title.as_ptr(), style, 100, 100, rc[2] - rc[0], rc[3] - rc[1], 0, 0, inst, 0);
            HWND.store(h, Ordering::Relaxed);
            ShowWindow(h, 5);
        }
    }
    pub fn pump() {
        unsafe {
            let mut m: Msg = std::mem::zeroed();
            while PeekMessageW(&mut m, 0, 0, 0, 1) != 0 {
                TranslateMessage(&m);
                DispatchMessageW(&m);
            }
        }
    }
    pub fn wait() {
        unsafe { WaitMessage() };
    }
    pub fn next_event() -> Option<Event> {
        EVENTS.lock().unwrap().pop_front()
    }
    pub fn close() {
        unsafe { DestroyWindow(HWND.load(Ordering::Relaxed)) };
    }
}

#[cfg(not(windows))]
mod host {
    use super::Event;
    pub fn init() {
        eprintln!("MANDELG (Rust) currently needs Windows");
        std::process::exit(1);
    }
    pub fn present(_: &[u8], _: &str) {}
    pub fn pump() {}
    pub fn wait() {}
    pub fn next_event() -> Option<Event> {
        None
    }
    pub fn close() {}
}

fn main() {
    host::init();
    let mut app = App::new();
    app.run();
    host::close();
}
