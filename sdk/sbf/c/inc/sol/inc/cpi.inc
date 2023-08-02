#pragma once
/**
 * @brief Domichain Cross-Program Invocation
 */

#include <domi/types.h>
#include <domi/pubkey.h>
#include <domi/entrypoint.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Maximum CPI instruction data size. 10 KiB was chosen to ensure that CPI
 * instructions are not more limited than transaction instructions if the size
 * of transactions is doubled in the future.
 */
static const uint64_t MAX_CPI_INSTRUCTION_DATA_LEN = 10240;

/**
 * Maximum CPI instruction accounts. 255 was chosen to ensure that instruction
 * accounts are always within the maximum instruction account limit for SBF
 * program instructions.
 */
static const uint8_t MAX_CPI_INSTRUCTION_ACCOUNTS = 255;

/**
 * Maximum number of account info structs that can be used in a single CPI
 * invocation. A limit on account info structs is effectively the same as
 * limiting the number of unique accounts. 128 was chosen to match the max
 * number of locked accounts per transaction (MAX_TX_ACCOUNT_LOCKS).
 */
static const uint16_t MAX_CPI_ACCOUNT_INFOS = 128;

/**
 * Account Meta
 */
typedef struct {
  DomiPubkey *pubkey; /** An account's public key */
  bool is_writable; /** True if the `pubkey` can be loaded as a read-write account */
  bool is_signer; /** True if an Instruction requires a Transaction signature matching `pubkey` */
} DomiAccountMeta;

/**
 * Instruction
 */
typedef struct {
  DomiPubkey *program_id; /** Pubkey of the instruction processor that executes this instruction */
  DomiAccountMeta *accounts; /** Metadata for what accounts should be passed to the instruction processor */
  uint64_t account_len; /** Number of DomiAccountMetas */
  uint8_t *data; /** Opaque data passed to the instruction processor */
  uint64_t data_len; /** Length of the data in bytes */
} DomiInstruction;

/**
 * Internal cross-program invocation function
 */
@SYSCALL uint64_t sol_invoke_signed_c(
  const DomiInstruction *,
  const DomiAccountInfo *,
  int,
  const DomiSignerSeeds *,
  int
);

/**
 * Invoke another program and sign for some of the keys
 *
 * @param instruction Instruction to process
 * @param account_infos Accounts used by instruction
 * @param account_infos_len Length of account_infos array
 * @param seeds Seed bytes used to sign program accounts
 * @param seeds_len Length of the seeds array
 */
static uint64_t sol_invoke_signed(
    const DomiInstruction *instruction,
    const DomiAccountInfo *account_infos,
    int account_infos_len,
    const DomiSignerSeeds *signers_seeds,
    int signers_seeds_len
) {
  return sol_invoke_signed_c(
    instruction,
    account_infos,
    account_infos_len,
    signers_seeds,
    signers_seeds_len
  );
}
/**
 * Invoke another program
 *
 * @param instruction Instruction to process
 * @param account_infos Accounts used by instruction
 * @param account_infos_len Length of account_infos array
*/
static uint64_t sol_invoke(
    const DomiInstruction *instruction,
    const DomiAccountInfo *account_infos,
    int account_infos_len
) {
  const DomiSignerSeeds signers_seeds[] = {{}};
  return sol_invoke_signed(
    instruction,
    account_infos,
    account_infos_len,
    signers_seeds,
    0
  );
}

#ifdef __cplusplus
}
#endif

/**@}*/