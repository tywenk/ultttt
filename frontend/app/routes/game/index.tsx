import { Route } from ".react-router/types/app/routes/game/+types";
import { wsService } from "@/lib/ws";
import { isMatch, isSnapshotResponse, Match, Team } from "@/types";
import { useEffect, useState } from "react";
import { Outlet } from "react-router";

export async function clientLoader({}: Route.ClientLoaderArgs) {
  const res = await fetch(`http://localhost:8000/api/matches/latest`);
  const match = (await res.json()) as Match;
  return { match };
}

export default function Index({ loaderData }: Route.ComponentProps) {
  const { match: initialMatch } = loaderData;

  const [team, setTeam] = useState<Team | null>(null);
  const [match, setMatch] = useState<Match | null>(initialMatch);
  const [snapshot, setSnapshot] = useState<number[][] | null>(null);

  useEffect(() => {
    const messageHandler = (rawData: string) => {
      try {
        const data = JSON.parse(rawData);
        if (isSnapshotResponse(data)) {
          setSnapshot(data.snap);
          setTeam(data.your_team);
        } else if (isMatch(data)) {
          setMatch(data);
        }
      } catch (e) {
        console.error(e);
      }
    };

    // Add the message handler before connecting
    wsService.subscribe(messageHandler);

    // Connect only if not already connected
    if (!wsService.isConnected) {
      wsService.connect();
    }

    // Cleanup
    return () => {
      wsService.unsubscribe(messageHandler);
    };
  }, []);
  return (
    <div className="w-full h-full">
      games route
      <Outlet />
    </div>
  );
}
