import { Route } from ".react-router/types/app/routes/game/+types";
import { Board } from "@/components/atoms/board";
import { Alert, AlertDescription, AlertTitle } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
  DialogTrigger,
} from "@/components/ui/dialog";
import { Progress } from "@/components/ui/progress";
import { cn } from "@/lib/utils";
import { wsService } from "@/lib/ws";
import {
  type Board as BoardT,
  isMatch,
  isSnapshotResponse,
  isTeamsResponse,
  isTimerResponse,
  Match,
  Status,
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
  const res = await fetch(
    `${import.meta.env.VITE_API_BASE_URL}/api/matches/latest`
  );
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

const clamp = (num: number, min: number, max: number) =>
  Math.min(Math.max(num, min), max);

export default function Index({ loaderData }: Route.ComponentProps) {
  const { match: initialMatch } = loaderData;

  const [myTeam, setMyTeam] = useState<Team | null>(null);
  const [match, setMatch] = useState<Match | null>(initialMatch);
  const [snapshot, setSnapshot] = useState<number[][] | null>(null);
  const [teamSizes, setTeamSize] = useState<number[] | null>(null);
  const [startTime, setStartTime] = useState<Date | null>(null);
  const [stopTime, setStopTime] = useState<Date | null>(null);
  const [timeRemaining, setTimeRemaining] = useState<number | null>(null);
  const [isPaused, setIsPaused] = useState<boolean>(false);

  const messageHandler = useCallback((rawData: string) => {
    try {
      const data = JSON.parse(rawData);
      if (isSnapshotResponse(data)) {
        setSnapshot(data.snap);
        // Only set team if it is not null
        if (data.your_team != null) setMyTeam(data.your_team);
      } else if (isTimerResponse(data)) {
        const start = new Date(data.start);
        const stop = new Date(data.stop);
        setStartTime(start);
        setStopTime(stop);
        setIsPaused(data.is_paused);
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

  // When time is set, calculate the time remaining and start count down
  useEffect(() => {
    if (startTime != null && stopTime != null) {
      let initialDiff = startTime.getTime() - stopTime.getTime();
      const interval = setInterval(() => {
        const diff = new Date().getTime() - stopTime.getTime();
        setTimeRemaining(clamp(Math.floor((diff / initialDiff) * 100), 0, 100));
      }, 500);
      return () => clearInterval(interval);
    }
  }, [startTime, stopTime]);

  const value = useMemo(() => {
    return {
      board: match?.board ?? null,
      snapshot,
      your_team: myTeam,
    } as MatchContextT;
  }, [match, myTeam, snapshot]);

  const gameIsComplete = match?.board?.status !== Status.Pending;
  const notEnoughPlayers = (teamSizes?.[0] ?? 0) + (teamSizes?.[1] ?? 0) < 2;
  const currTeam = match?.board.current_team;
  const boardStatus = match?.board.status;

  return (
    <MatchProvider value={value}>
      <div className="w-full mx-auto h-full min-h-screen max-w-prose flex flex-col gap-2 items-center pt-6">
        <div className="w-full items-center px-12 py-4 flex gap-2 flex-col">
          <h1 className="font-medium text-xl">Ultimate Tic Tac Toe MMO</h1>
          <div className="flex flex-col sm:flex-row gap-2">
            <Badge variant={myTeam == Team.O ? "teamO" : "teamX"}>
              You are on team:
              {myTeam === Team.O ? (
                <Circle className="ml-2 h-4 w-4 inline-block" />
              ) : myTeam === Team.X ? (
                <X className="ml-2 h-4 w-4 inline-block" />
              ) : (
                "No team"
              )}
            </Badge>
            <Dialog>
              <DialogTrigger asChild>
                <Button size="sm" variant="outline">
                  <CircleHelp />
                  How to play
                </Button>
              </DialogTrigger>
              <DialogContent>
                <DialogHeader>
                  <DialogTitle>How to play</DialogTitle>
                  <DialogDescription>
                    The first team to win three small Tic Tac Toe games in a row
                    wins the big game.
                  </DialogDescription>
                  <DialogDescription>
                    You can only play in the small Tic Tac Toe game that is
                    selected by the previous player. For example, If your
                    opponent selects the bottom right corner of the small Tic
                    Tac Toe game, you must play in the bottom right corner of
                    the big Tic Tac Toe game.
                  </DialogDescription>
                  <DialogDescription>
                    The selected game you are allowed to play in is highlighted
                    in yellow.
                  </DialogDescription>
                  <DialogDescription>Good luck and have fun!</DialogDescription>
                  <DialogDescription>
                    This is a multiplayer game! Vote for the next move by
                    selecting the cell you want your team to play in. You can
                    select as many times as you want.
                  </DialogDescription>
                </DialogHeader>
              </DialogContent>
            </Dialog>
          </div>
        </div>
        <div
          className={cn(
            "w-full h-10 items-center flex justify-center",
            gameIsComplete && "h-28"
          )}
        >
          {isPaused && notEnoughPlayers ? (
            <span className="text-xs">
              Game paused. Waiting for more players to join.
            </span>
          ) : isPaused && gameIsComplete ? (
            <Alert className="w-1/2">
              <AlertTitle>
                {boardStatus === Status.Tied
                  ? "Game tied!"
                  : boardStatus === Status.X
                  ? "Team X wins!"
                  : boardStatus === Status.O
                  ? "Team O wins!"
                  : "Game over!"}
              </AlertTitle>
              <AlertDescription>
                Starting new game in 30 seconds...
                <div className="w-full flex p-1">
                  <Progress className={cn("w-full")} value={timeRemaining} />
                </div>
              </AlertDescription>
            </Alert>
          ) : timeRemaining == 0 &&
            snapshot?.every((row) => row.every((cell) => cell === 0)) ? (
            <span className="text-xs">
              Waiting for team {currTeam == Team.O ? "O" : "X"} to vote
            </span>
          ) : (
            <Progress
              className={cn(
                "w-1/2",
                currTeam === Team.O && "[&>*]:bg-blue-800",
                currTeam === Team.X && "[&>*]:bg-rose-800"
              )}
              value={timeRemaining}
            />
          )}
        </div>
        {match?.board != null && <Board board={match.board} />}
        <div className="flex flex-col items-center md:justify-between md:gap-6 md:flex-row gap-2">
          <Badge
            className="w-fit"
            variant={currTeam == Team.O ? "teamO" : "teamX"}
          >
            Current turn: {currTeam?.toLocaleUpperCase() ?? "No team"}
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
