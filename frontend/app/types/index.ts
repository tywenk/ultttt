export enum Status {
  X = "x",
  O = "o",
  Tied = "tied",
  Pending = "pending",
}

export enum Player {
  X = "x",
  O = "o",
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
  current_player: Player;
};
