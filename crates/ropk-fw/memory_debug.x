/*
debug layout, to be used with a debug probe, nukes the softdevice and bootloader
DO NOT MAKE A UF2 FROM THIS
use ./memory_prod.x to build the uf2
 */

MEMORY
{
  FLASH : ORIGIN = 0x00000000, LENGTH = 1024K
  RAM   : ORIGIN = 0x20000000, LENGTH = 256K
}
