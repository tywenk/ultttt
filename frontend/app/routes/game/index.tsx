import { Route } from ".react-router/types/app/routes/game/+types";
import { Board } from "@/components/atoms/board";
import { Badge } from "@/components/ui/badge";
import { wsService } from "@/lib/ws";
import {
  type Board as BoardT,
  isMatch,
  isSnapshotResponse,
  isTeamsResponse,
  Match,
  Team,
} from "@/types";
import {
  createContext,
  useCallback,
  useEffect,
  useMemo,
  useState,
} from "react";

export async function clientLoader({}: Route.ClientLoaderArgs) {
  const res = await fetch(`http://localhost:8000/api/matches/latest`);
  const match = (await res.json()) as Match;
  return { match };
}

export type MatchContextT = {
  board: BoardT;
  snapshot: number[][];
  your_team: Team;
};
export const MatchContext = createContext<MatchContextT | null>(null);
const MatchProvider = MatchContext.Provider;

export default function Index({ loaderData }: Route.ComponentProps) {
  const { match: initialMatch } = loaderData;

  const [myTeam, setMyTeam] = useState<Team | null>(null);
  const [match, setMatch] = useState<Match | null>(initialMatch);
  const [snapshot, setSnapshot] = useState<number[][] | null>(null);
  const [teamSizes, setTeamSize] = useState<number[] | null>(null);
  console.log({ snapshot, match });

  const messageHandler = useCallback((rawData: string) => {
    try {
      const data = JSON.parse(rawData);
      console.log("WEBSOCKET: ", data);
      if (isSnapshotResponse(data)) {
        setSnapshot(data.snap);
        // Only set team if it is not null
        if (data.your_team != null) setMyTeam(data.your_team);
      } else if (isTeamsResponse(data)) {
        setTeamSize([data.x_team_size, data.o_team_size]);
      } else if (isMatch(data)) {
        setMatch(data);
      }
    } catch (e) {
      console.error(e);
    }
  }, []);

  useEffect(() => {
    wsService.subscribe(messageHandler);
    if (!wsService.isConnected) {
      wsService.connect();
    }
    return () => {
      wsService.unsubscribe(messageHandler);
    };
  }, []);

  const value = useMemo(() => {
    return {
      board: match?.board ?? null,
      snapshot,
      your_team: myTeam,
    } as MatchContextT;
  }, [match, myTeam]);

  return (
    <MatchProvider value={value}>
      <div className="w-full mx-auto h-full min-h-screen max-w-prose flex flex-col gap-2 items-center pt-6">
        <div className="w-full px-12 py-4 flex gap-2 flex-col">
          <h1 className="font-medium text-xl">
            Welcome to Ultimate Tic Tac Toe MMO
          </h1>
          <div className="flex flex-row gap-2">
            <Badge variant={myTeam == Team.O ? "teamO" : "teamX"}>
              You are on team: {myTeam?.toLocaleUpperCase() ?? "No team"}
            </Badge>
            <Badge
              variant={match?.board.current_team == Team.O ? "teamO" : "teamX"}
            >
              Current turn:{" "}
              {match?.board.current_team?.toLocaleUpperCase() ?? "No team"}
            </Badge>
            <Badge variant="outline">X Players: {teamSizes?.[0]}</Badge>
            <Badge variant="outline">O Players: {teamSizes?.[1]}</Badge>
          </div>
        </div>
        {match?.board != null && <Board board={match.board} />}
      </div>
    </MatchProvider>
  );
}
