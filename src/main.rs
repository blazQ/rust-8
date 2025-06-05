use rust_8::Chip8;
// da linea di comando bisogna chiamare l'eseguibile in questo modo:
// cargo run file.o8

fn main() {
    // leggi il file contenente la rom
    // istanzia un emulatore per quella rom?
    let file_path = "test_roms\\Maze [David Winter, 199x].ch8";

    //let message= fs::read(file_path);
    //message.unwrap().chunks(2).for_each(|pair| println!("{:02x} {:02x}", pair[0], pair[1]));

    let emulator = Chip8::new();
    emulator
        .load_rom(file_path)
        .unwrap()
        .run()
        .expect("Error during run");
}
