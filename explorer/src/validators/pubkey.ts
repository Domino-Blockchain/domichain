import { coerce, instance, string } from "superstruct";
import { PublicKey } from "@domichain/web3.js";

export const PublicKeyFromString = coerce(
  instance(PublicKey),
  string(),
  (value) => new PublicKey(value)
);
