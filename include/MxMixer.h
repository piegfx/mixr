#ifndef MIXR_MXMIXER_H
#define MIXR_MXMIXER_H

#include <stdint.h>

typedef void* MxMixer;

void mxCreateMixer(uint32_t numVoices, uint32_t sampleRate, MxMixer* mixer);

#endif
