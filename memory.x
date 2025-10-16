/* Memory layout for Raspberry Pi RP2040 */
MEMORY {
    /* Boot ROM (not directly accessible) */
    BOOT2 : ORIGIN = 0x10000000, LENGTH = 0x100

    /* Flash memory - 2MB total, boot2 takes first 256 bytes */
    FLASH : ORIGIN = 0x10000100, LENGTH = 2048K - 0x100

    /* RAM - 264KB total */
    RAM   : ORIGIN = 0x20000000, LENGTH = 264K
}

/* Place boot2 in the first 256 bytes of flash */
SECTIONS {
    .boot2 ORIGIN(BOOT2) :
    {
        KEEP(*(.boot2));
    } > BOOT2
} INSERT BEFORE .text;

/* The entry point is the reset handler */
EXTERN(RESET_VECTOR);
