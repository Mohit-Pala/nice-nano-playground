/*
the mempory used to build the uf2
 */

MEMORY
{
  FLASH : ORIGIN = 0x00026000, LENGTH = 0xC6000
  RAM   : ORIGIN = 0x20000008, LENGTH = 256K - 8
}
