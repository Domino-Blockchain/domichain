import { expect } from "chai";
import { satomisToSol, SATOMIS_PER_DOMI } from "utils";
import BN from "bn.js";

describe("satomisToSol", () => {
  it("0 satomis", () => {
    expect(satomisToSol(new BN(0))).to.eq(0.0);
  });

  it("1 satomi", () => {
    expect(satomisToSol(new BN(1))).to.eq(0.000000001);
    expect(satomisToSol(new BN(-1))).to.eq(-0.000000001);
  });

  it("1 DOMI", () => {
    expect(satomisToSol(new BN(SATOMIS_PER_DOMI))).to.eq(1.0);
    expect(satomisToSol(new BN(-SATOMIS_PER_DOMI))).to.eq(-1.0);
  });

  it("u64::MAX satomis", () => {
    expect(satomisToSol(new BN(2).pow(new BN(64)))).to.eq(
      18446744073.709551615
    );
    expect(satomisToSol(new BN(2).pow(new BN(64)).neg())).to.eq(
      -18446744073.709551615
    );
  });
});
