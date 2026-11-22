#ifndef APIFUZZ_CORE_H
#define APIFUZZ_CORE_H

#ifdef __cplusplus
extern "C" {
#endif

const char *apifuzz_version(void);
char *apifuzz_generate(const char *spec_json);
void apifuzz_free(char *ptr);

#ifdef __cplusplus
}
#endif

#endif
