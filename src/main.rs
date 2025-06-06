use crossterm::{
    event::{Event, KeyCode, poll, read},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::env;
use std::thread;
use std::time::{Duration, Instant};
use rust_8::Chip8;

struct Config {
    rom_path: String,
    cpu_freq: u32,
}

impl Config {
    fn from_args() -> Result<Config, String> {
        let args: Vec<String> = env::args().collect();
        
        let mut rom_path = String::from("test_roms\\tetris.ch8");
        let mut cpu_freq = 700;
        
        let mut i = 1;
        while i < args.len() {
            match args[i].as_str() {
                "--rom" => {
                    if i + 1 < args.len() {
                        rom_path = args[i + 1].clone();
                        i += 2;
                    } else {
                        return Err("--nomerom requires a ROM path".to_string());
                    }
                }
                "--tickcpu" | "--cpu" => {
                    if i + 1 < args.len() {
                        cpu_freq = args[i + 1].parse()
                            .map_err(|_| "Invalid CPU frequency value".to_string())?;
                        i += 2;
                    } else {
                        return Err("--tickcpu requires a frequency value".to_string());
                    }
                }
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => {
                    // Se non è un flag, assumiamo sia il nome della ROM
                    if !args[i].starts_with("--") {
                        rom_path = args[i].clone();
                    }
                    i += 1;
                }
            }
        }
        
        Ok(Config { rom_path, cpu_freq })
    }
}

fn print_help() {
    println!("CHIP-8 Emulator");
    println!("Usage: cargo run [OPTIONS] [ROM_PATH]");
    println!();
    println!("OPTIONS:");
    println!("  --rom <PATH>     ROM file to load (default: test_roms\\tetris.ch8)");
    println!("  --tickcpu, --cpu <FREQ>     CPU frequency in Hz (default: 700)");
    println!("  --help, -h                  Show this help message");
    println!();
    println!("EXAMPLES:");
    println!("  cargo run                                    # Run with default ROM and settings");
    println!("  cargo run my_game.ch8                       # Run specific ROM");
    println!("  cargo run --nomerom pong.ch8 --tickcpu 1000 # Run with custom ROM and CPU speed");
    println!("  cargo run --cpu 500                         # Run with slower CPU");
    println!();
    println!("KEYBOARD LAYOUT:");
    println!("  CHIP-8:     Keyboard:");
    println!("  1 2 3 C     1 2 3 4");
    println!("  4 5 6 D  →  Q W E R");
    println!("  7 8 9 E     A S D F");
    println!("  A 0 B F     Z X C V");
    println!();
    println!("Press ESC to exit the emulator.");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_args().map_err(|e| {
        eprintln!("Error: {}", e);
        eprintln!("Use --help for usage information.");
        std::process::exit(1);
    })?;
    
    println!("Loading ROM: {}", config.rom_path);
    println!("CPU Frequency: {} Hz", config.cpu_freq);
    
    let mut chip8 = Chip8::new().load_rom(&config.rom_path)?;
    
    enable_raw_mode()?;
    
    let mut last_cpu_time = Instant::now();
    let mut last_timer_time = Instant::now();
    let mut last_display_time = Instant::now();
    
    let cpu_freq = Duration::from_nanos(1_000_000_000 / config.cpu_freq as u64);
    let timer_freq = Duration::from_nanos(1_000_000_000 / 60); // 60Hz timers
    let display_freq = Duration::from_millis(16); // ~60 FPS display
    
    println!("Starting emulator... Press ESC to exit.");
    
    'main: loop {
        let frame_start = Instant::now();
        
        // 1. Reset keyboard every frame
        chip8.keyboard.fill(false);
        
        // 2. Handle input events
        while poll(Duration::from_millis(1))? {
            match read()? {
                Event::Key(key_event) => match key_event.code {
                    KeyCode::Esc => break 'main,
                    KeyCode::Char('1') => chip8.keyboard[0x1] = true,
                    KeyCode::Char('2') => chip8.keyboard[0x2] = true,
                    KeyCode::Char('3') => chip8.keyboard[0x3] = true,
                    KeyCode::Char('4') => chip8.keyboard[0xC] = true,
                    KeyCode::Char('q') => chip8.keyboard[0x4] = true,
                    KeyCode::Char('w') => chip8.keyboard[0x5] = true,
                    KeyCode::Char('e') => chip8.keyboard[0x6] = true,
                    KeyCode::Char('r') => chip8.keyboard[0xD] = true,
                    KeyCode::Char('a') => chip8.keyboard[0x7] = true,
                    KeyCode::Char('s') => chip8.keyboard[0x8] = true,
                    KeyCode::Char('d') => chip8.keyboard[0x9] = true,
                    KeyCode::Char('f') => chip8.keyboard[0xE] = true,
                    KeyCode::Char('z') => chip8.keyboard[0xA] = true,
                    KeyCode::Char('x') => chip8.keyboard[0x0] = true,
                    KeyCode::Char('c') => chip8.keyboard[0xB] = true,
                    KeyCode::Char('v') => chip8.keyboard[0xF] = true,
                    _ => {}
                },
                _ => {}
            }
        }
        
        if last_cpu_time.elapsed() >= cpu_freq {
            let ticks = (last_cpu_time.elapsed().as_nanos() / cpu_freq.as_nanos()) as usize;
            if let Err(e) = chip8.run(ticks.min(10)) { // Max 10 ticks per frame
                eprintln!("CPU Error: {}", e);
                break;
            }
            last_cpu_time = Instant::now();
        }
        
        if last_timer_time.elapsed() >= timer_freq {
            chip8.tick_timers();
            last_timer_time = Instant::now();
        }
        
        if last_display_time.elapsed() >= display_freq {
            if chip8.should_update_display() {
                chip8.print_display();
            }
            last_display_time = Instant::now();
        }
        
        let elapsed = frame_start.elapsed();
        if elapsed < Duration::from_millis(1) {
            thread::sleep(Duration::from_millis(1) - elapsed);
        }
    }
    
    disable_raw_mode()?;
    println!("Emulator stopped.");
    Ok(())
}