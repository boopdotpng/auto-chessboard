"""Generate long algebraic move listings with FEN snapshots from PGN games.

Usage:
	python gen_long.py Fischer.pgn output.txt --max-games 250

The script writes each move in long algebraic notation followed by the FEN
after the move, separated by newlines, and terminates each game section with
`---`.
"""

from __future__ import annotations

import argparse
from pathlib import Path

import chess  # type: ignore

try:
	import chess.pgn  # type: ignore
except ImportError as exc:  # pragma: no cover - environment specific
	raise SystemExit("python-chess is required to run this script.") from exc


def format_move(board: chess.Board, move: chess.Move) -> str:
	"""Return a hyphenated coordinate move string (castles split into two)."""

	if board.is_castling(move):
		king_move = f"{chess.square_name(move.from_square)}-{chess.square_name(move.to_square)}"

		if board.turn == chess.WHITE:
			if move.to_square == chess.G1:
				rook_from, rook_to = chess.H1, chess.F1
			else:
				rook_from, rook_to = chess.A1, chess.D1
		else:
			if move.to_square == chess.G8:
				rook_from, rook_to = chess.H8, chess.F8
			else:
				rook_from, rook_to = chess.A8, chess.D8

		rook_move = f"{chess.square_name(rook_from)}-{chess.square_name(rook_to)}"
		return f"{king_move},{rook_move}"

	move_str = (
		f"{chess.square_name(move.from_square)}-{chess.square_name(move.to_square)}"
	)

	if move.promotion:
		move_str += f"={chess.piece_symbol(move.promotion).upper()}"

	return move_str


def convert_pgn_to_long_format(
	pgn_path: Path, output_path: Path, max_games: int
) -> int:
	"""Convert up to *max_games* from the PGN into the target output format."""
	games_written = 0
	output_lines: list[str] = []

	with pgn_path.open("r", encoding="utf-8") as pgn_file:
		while games_written < max_games:
			game = chess.pgn.read_game(pgn_file)
			if game is None:
				break

			board = game.board()
			for move in game.mainline_moves():
				move_repr = format_move(board, move)
				board.push(move)
				output_lines.append(move_repr)
				output_lines.append(board.fen())

			output_lines.append("---")
			games_written += 1

	if games_written == 0:
		return 0

	output_path.write_text("\n".join(output_lines) + "\n", encoding="utf-8")
	return games_written


def build_parser() -> argparse.ArgumentParser:
	parser = argparse.ArgumentParser(
		description="Convert PGN games to long algebraic notation with FENs."
	)
	parser.add_argument("pgn", type=Path, help="Path to the input PGN file.")
	parser.add_argument(
		"output",
		type=Path,
		help="Path to the output text file that will be created/overwritten.",
	)
	parser.add_argument(
		"--max-games",
		type=int,
		default=250,
		help="Maximum number of games to convert (default: 250).",
	)
	return parser


def main() -> None:
	parser = build_parser()
	args = parser.parse_args()

	if args.max_games <= 0:
		parser.error("--max-games must be a positive integer")

	games_written = convert_pgn_to_long_format(args.pgn, args.output, args.max_games)

	if games_written == 0:
		parser.exit(1, "No games were found in the provided PGN.\n")


if __name__ == "__main__":
	main()
