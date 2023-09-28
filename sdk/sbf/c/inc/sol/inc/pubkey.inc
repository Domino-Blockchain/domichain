#pragma once
/**
 * @brief Domichain Public key
 */

#include <domi/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Size of Public key in bytes
 */
#define SIZE_PUBKEY 32

/**
 * Public key
 */
typedef struct {
  uint8_t x[SIZE_PUBKEY];
} DomiPubkey;

/**
 * Prints the hexadecimal representation of a public key
 *
 * @param key The public key to print
 */
@SYSCALL void sol_log_pubkey(const DomiPubkey *);

/**
 * Compares two public keys
 *
 * @param one First public key
 * @param two Second public key
 * @return true if the same
 */
static bool DomiPubkey_same(const DomiPubkey *one, const DomiPubkey *two) {
  for (int i = 0; i < sizeof(*one); i++) {
    if (one->x[i] != two->x[i]) {
      return false;
    }
  }
  return true;
}

/**
 * Seed used to create a program address or passed to sol_invoke_signed
 */
typedef struct {
  const uint8_t *addr; /** Seed bytes */
  uint64_t len; /** Length of the seed bytes */
} DomiSignerSeed;

/**
 * Seeds used by a signer to create a program address or passed to
 * sol_invoke_signed
 */
typedef struct {
  const DomiSignerSeed *addr; /** An array of a signer's seeds */
  uint64_t len; /** Number of seeds */
} DomiSignerSeeds;

/**
 * Create a program address
 *
 * @param seeds Seed bytes used to sign program accounts
 * @param seeds_len Length of the seeds array
 * @param program_id Program id of the signer
 * @param program_address Program address created, filled on return
 */
@SYSCALL uint64_t sol_create_program_address(const DomiSignerSeed *, int, const DomiPubkey *, DomiPubkey *);

/**
 * Try to find a program address and return corresponding bump seed
 *
 * @param seeds Seed bytes used to sign program accounts
 * @param seeds_len Length of the seeds array
 * @param program_id Program id of the signer
 * @param program_address Program address created, filled on return
 * @param bump_seed Bump seed required to create a valid program address
 */
@SYSCALL uint64_t sol_try_find_program_address(const DomiSignerSeed *, int, const DomiPubkey *, DomiPubkey *, uint8_t *);

#ifdef DOMI_TEST
/**
 * Stub functions when building tests
 */
#include <stdio.h>

void sol_log_pubkey(
  const DomiPubkey *pubkey
) {
  printf("Program log: ");
  for (int i = 0; i < SIZE_PUBKEY; i++) {
    printf("%02 ", pubkey->x[i]);
  }
  printf("\n");
}

#endif

#ifdef __cplusplus
}
#endif

/**@}*/
