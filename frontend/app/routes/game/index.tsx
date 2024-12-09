import { Route } from ".react-router/types/app/routes/game/+types";
import { Board } from "@/components/atoms/board";
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

type MatchContextT = {
  board: BoardT | null;
  your_team: Team | null;
};
const MatchContext = createContext<MatchContextT | null>(null);
const MatchProvider = MatchContext.Provider;

export default function Index({ loaderData }: Route.ComponentProps) {
  const { match: initialMatch } = loaderData;

  const [team, setTeam] = useState<Team | null>(null);
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
        if (data.your_team != null) setTeam(data.your_team);
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
      your_team: team,
    } as MatchContextT;
  }, [match, team]);

  return (
    <MatchProvider value={value}>
      <div className="w-full max-w-prose h-full">
        <p>Your team: {team}</p>
        <p>X team size: {teamSizes?.[0]}</p>
        <p>O team size: {teamSizes?.[1]}</p>
        {match?.board != null && <Board board={match.board} />}
        <p>Current Turn: {match?.board.current_team}</p>
      </div>
    </MatchProvider>
  );
}
