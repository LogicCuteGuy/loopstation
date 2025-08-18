/* Memory layout for STM32H743VIT6 */
MEMORY
{
  /* Flash memory - 2MB total */
  FLASH : ORIGIN = 0x08000000, LENGTH = 2048K
  
  /* RAM - STM32H743VIT6 has multiple RAM regions */
  /* DTCM RAM - 128KB, fastest access for data */
  DTCMRAM : ORIGIN = 0x20000000, LENGTH = 128K
  
  /* AXI SRAM - 512KB, main system RAM */
  RAM : ORIGIN = 0x24000000, LENGTH = 512K
  
  /* SRAM1 - 128KB */
  SRAM1 : ORIGIN = 0x30000000, LENGTH = 128K
  
  /* SRAM2 - 128KB */
  SRAM2 : ORIGIN = 0x30020000, LENGTH = 128K
  
  /* SRAM3 - 32KB */
  SRAM3 : ORIGIN = 0x30040000, LENGTH = 32K
  
  /* SRAM4 - 64KB, backup domain */
  SRAM4 : ORIGIN = 0x38000000, LENGTH = 64K
}

/* Stack starts at end of DTCM RAM */
_stack_start = ORIGIN(DTCMRAM) + LENGTH(DTCMRAM);

/* Audio buffers will be placed in AXI SRAM for DMA access */
_audio_buffer_start = ORIGIN(RAM);
_audio_buffer_size = 256K; /* Reserve 256KB for audio buffers */