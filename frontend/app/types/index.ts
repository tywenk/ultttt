export enum Status {
  X = "x",
  O = "o",
  Tied = "tied",
  Pending = "pending",
}

export enum Team {
  X = "x",
  O = "o",
}

export function isTeam(data: any): data is Team {
  return data === Team.X || data === Team.O;
}

export type Cell = {
  status: Status;
};

export type Section = {
  data: Cell[];
  status: Status;
  is_interactive: boolean;
};

export type Board = {
  data: Section[];
  status: Status;
  current_team: Team;
};

export type Match = {
  id: string;
  board: Board;
  created_at: string;
  updated_at: string;
};

export function isMatch(data: any): data is Match {
  return (
    typeof data === "object" &&
    data !== null &&
    "id" in data &&
    "board" in data &&
    "created_at" in data &&
    "updated_at" in data
  );
}

export type SnapshotResponse = {
  your_team: Team | null;
  snap: number[][];
};

export function isSnapshotResponse(data: any): data is SnapshotResponse {
  return (
    typeof data === "object" &&
    data !== null &&
    "your_team" in data &&
    "snap" in data
  );
}

export type TeamsResponse = {
  x_team_size: number;
  o_team_size: number;
};

export function isTeamsResponse(data: any): data is TeamsResponse {
  return (
    typeof data === "object" &&
    data !== null &&
    "x_team_size" in data &&
    "o_team_size" in data
  );
}
