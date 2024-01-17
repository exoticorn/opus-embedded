#ifndef FASTFUNC_H_
#define FASTFUNC_H_

#ifdef CODE_IN_RAM
#define FAST_FUNC __attribute__ ((section (".data")))
#else
#define FAST_FUNC
#endif

#endif
