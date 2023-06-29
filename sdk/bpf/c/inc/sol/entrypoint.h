#pragma once
/**
 * @brief Domichain program entrypoint
 */

#include <domi/constants.h>
#include <domi/types.h>
#include <domi/pubkey.h>

#ifdef __cplusplus
extern "C" {
#endif

/**
 * Keyed Account
 */
typedef struct {
  DomiPubkey *key;      /** Public key of the account */
  uint64_t *lamports;  /** Number of lamports owned by this account */
  uint64_t data_len;   /** Length of data in bytes */
  uint8_t *data;       /** On-chain data within this account */
  DomiPubkey *owner;    /** Program that owns this account */
  uint64_t rent_epoch; /** The epoch at which this account will next owe rent */
  bool is_signer;      /** Transaction was signed by this account's key? */
  bool is_writable;    /** Is the account writable? */
  bool executable;     /** This account's data contains a loaded program (and is now read-only) */
} DomiAccountInfo;

/**
 * Structure that the program's entrypoint input data is deserialized into.
 */
typedef struct {
  DomiAccountInfo* ka; /** Pointer to an array of DomiAccountInfo, must already
                          point to an array of DomiAccountInfos */
  uint64_t ka_num; /** Number of DomiAccountInfo entries in `ka` */
  const uint8_t *data; /** pointer to the instruction data */
  uint64_t data_len; /** Length in bytes of the instruction data */
  const DomiPubkey *program_id; /** program_id of the currently executing program */
} DomiParameters;

/**
 * Program instruction entrypoint
 *
 * @param input Buffer of serialized input parameters.  Use domi_deserialize() to decode
 * @return 0 if the instruction executed successfully
 */
uint64_t entrypoint(const uint8_t *input);

#ifdef __cplusplus
}
#endif

/**@}*/
