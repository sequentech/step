export const getRandomNumberBetween = (n: number, m: number) => {
	return Math.floor(Math.random() * (m - n + 1)) + n;
}