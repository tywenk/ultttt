import { Button } from "@/components/ui/button";
import { type Cell } from "@/types";

export function Cell({ cell }: { cell: Cell }) {
  return (
    <Button className="">
      {cell.status === "x" ? "X" : cell.status === "o" ? "O" : "-"}
    </Button>
  );
}
