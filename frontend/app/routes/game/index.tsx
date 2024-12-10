import { Route } from ".react-router/types/app/routes/game/+types";
import { Board } from "@/components/atoms/board";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogTrigger } from "@/components/ui/dialog";
import { wsService } from "@/lib/ws";
import {
  type Board as BoardT,
  isMatch,
  isSnapshotResponse,
  isTeamsResponse,
  Match,
  Team,
} from "@/types";
import { Circle, CircleHelp, Users, X } from "lucide-react";
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

  const messageHandler = useCallback((rawData: string) => {
    try {
      const data = JSON.parse(rawData);
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
  }, [match, myTeam, snapshot]);

  return (
    <MatchProvider value={value}>
      <div className="w-full mx-auto h-full min-h-screen max-w-prose flex flex-col gap-2 items-center pt-6">
        <div className="w-full items-center px-12 py-4 flex gap-2 flex-col">
          <h1 className="font-medium text-xl">Ultimate Tic Tac Toe MMO</h1>
          <div className="flex flex-col md:flex-row gap-2">
            <Badge variant={myTeam == Team.O ? "teamO" : "teamX"}>
              You are on team:{" "}
              {myTeam === Team.O ? (
                <Circle className="h-4 w-4 inline-block" />
              ) : myTeam === Team.X ? (
                <X className="h-4 w-4 inline-block" />
              ) : (
                "No team"
              )}
            </Badge>
            <Dialog>
              <DialogTrigger>
                <Button size="sm" variant="outline">
                  <CircleHelp />
                  How to play
                </Button>
              </DialogTrigger>
              <DialogContent>
                <h2 className="font-medium text-lg">How to play</h2>
                <p>
                  The first team to win three small Tic Tac Toe games in a row
                  wins the big game.
                </p>
                <p>
                  This is a multiplayer game! Vote for the next move by
                  selecting the cell you want your team to play in. You can
                  select as many times as you want.
                </p>
                <p>
                  You can only play in the small Tic Tac Toe game that is
                  selected by the previous player. For example, If your opponent
                  selects the bottom right corner of the small Tic Tac Toe game,
                  you must play in the bottom right corner of the big Tic Tac
                  Toe game.
                </p>
                <p>
                  The selected game you are allowed to play in is highlighted in
                  yellow.
                </p>
                <p>Good luck and have fun!</p>
              </DialogContent>
            </Dialog>
          </div>
        </div>
        {match?.board != null && <Board board={match.board} />}
        <div className="flex flex-col items-center md:justify-between md:gap-6 md:flex-row gap-2">
          <Badge
            className="w-fit"
            variant={match?.board.current_team == Team.O ? "teamO" : "teamX"}
          >
            Current turn:{" "}
            {match?.board.current_team?.toLocaleUpperCase() ?? "No team"}
          </Badge>
          <div className="flex gap-2 items-center">
            <Users className="h-4 w-4 inline-block" />
            <Badge variant="outline">
              <X className="mr-1 h-4 w-4 block" />
              {teamSizes?.[0].toLocaleString()}
            </Badge>
            <Badge variant="outline">
              <Circle className="mr-1 h-4 w-4 block" />
              {teamSizes?.[1].toLocaleString()}
            </Badge>
          </div>
        </div>
      </div>
    </MatchProvider>
  );
}
