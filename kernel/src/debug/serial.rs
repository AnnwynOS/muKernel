const SERIAL_PORT: u16 = 0x3F8;

pub struct Serial;

impl Serial {
    pub fn init() {
        unsafe {
            use x86_64::instructions::port::Port;

            // Offset plus 1 : Registre des interruptions
            // Bit 0 : interuptions désactivées ; Bit 1 : interruptions activées
            // 00
            Port::new(SERIAL_PORT + 1).write(0x00u8);

            // En offset plus 3 : Registre de contrôle de ligne, LCR
            // Bit 0 ou 1 : données ; bit 2 : stop ; bits 3 à cinq : parité ; Bit six : break enable ; Bit 7 : DLAB
            // 10000000
            Port::new(SERIAL_PORT + 3).write(0x80u8);

            // 115200 / 3 à 38400 bauds.
            // Comme DLAB est à 1, l'accès à 0 setup le DLL et l'accès à 1 le DLH
            Port::new(SERIAL_PORT + 0).write(0x03u8); // Diviseur bas DLl
            Port::new(SERIAL_PORT + 1).write(0x00u8); // Diviseur haut DLH

            // 8 bits, pas de parité, 1 stop bit. Désactivation du DLAB
            // 00000011
            Port::new(SERIAL_PORT + 3).write(0x03u8);

            // En offset 2 : configurer les fifos
            // Bit 0 : Activer ; Bit 1 : supprimer le tempon en réception ; Bit 2 : en envoi ; Bit 3 : DMA ; Bit 4 cinq : réservé ; Bit six 7 : interrupt trigger level
            // 11000111
            // Donc ITT à 14 bits ; on commence fifo et on cleare tout.
            Port::new(SERIAL_PORT + 2).write(0xC7u8);

            // En offset4: configurer le modem
            // bit 0 : config du PIN du data terminal READY DTR ; Bit 1 : pin du request to send RTS ; Bit 2: OUT1 ; Bit 3: OUT2 ; Bit 4 : Loop. Suite inusitée
            // 00001011
            // Donc en OUT2
            Port::new(SERIAL_PORT + 4).write(0x0Bu8);
        }
    }

    fn is_ready() -> bool {
        unsafe {
            use x86_64::instructions::port::Port;
            let mut port: Port<u8> = Port::new(SERIAL_PORT + 5);
            // Vérifier que le cinquième bit, THR Empty est à 1
            (port.read() & 0x20) != 0
        }
    }

    pub fn write_byte(byte: u8) {
        while !Self::is_ready() {}
        unsafe {
            use x86_64::instructions::port::Port;
            let mut port = Port::new(SERIAL_PORT);
            port.write(byte);
        }
    }
}