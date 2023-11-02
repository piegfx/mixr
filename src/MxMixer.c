#include "MxMixer.h"

#include <malloc.h>

typedef struct {

} MxVoice;

typedef struct {
    MxVoice* Voices;
} MxMixerImpl;

void mxCreateMixer(uint32_t numVoices, uint32_t sampleRate, MxMixer* mixer) {
    MxVoice* voices = (MxVoice*) malloc(numVoices);
}