import { DomichainPingProvider } from "providers/stats/DomichainPingProvider";
import React from "react";
import { DomichainClusterStatsProvider } from "./solanaClusterStats";

type Props = { children: React.ReactNode };
export function StatsProvider({ children }: Props) {
  return (
    <DomichainClusterStatsProvider>
      <DomichainPingProvider>{children}</DomichainPingProvider>
    </DomichainClusterStatsProvider>
  );
}
