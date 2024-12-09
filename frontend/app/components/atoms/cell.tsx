import { Button } from "@/components/ui/button";
import { wsService } from "@/lib/ws";
import { Status, type Cell } from "@/types";

export function Cell({ cell, location }: { cell: Cell; location: number[] }) {
  const { status } = cell;

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
      onClick={handleClick}
      className="aspect-square min-w-[40px] min-h-[40px] w-full font-mono"
    >
      {status === Status.O ? "O" : status === Status.X ? "X" : " "}
    </Button>
  );
}
