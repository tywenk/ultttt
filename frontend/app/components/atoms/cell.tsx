import { Button } from "@/components/ui/button";
import { wsService } from "@/lib/ws";
import { MatchContext } from "@/routes/game";
import { Status, type Cell } from "@/types";
import { useContext } from "react";

export function Cell({
  cell,
  disabled: disabledProp,
  location,
}: {
  cell: Cell;
  disabled: boolean;
  location: number[];
}) {
  const context = useContext(MatchContext);
  const { status } = cell;

  const disabled = cell.status !== Status.Pending || disabledProp;

  const handleClick = () => {
    if (location.length != 2) return;
    wsService.send(
      JSON.stringify({
        section: location[0],
        cell: location[1],
      })
    );
  };

  return (
    <Button
      variant="outline"
      size="icon"
      onClick={handleClick}
      className=""
      disabled={disabled}
    >
      {status === Status.O ? "O" : status === Status.X ? "X" : " "}
    </Button>
  );
}
