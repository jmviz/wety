export default async function handler(req, res) {
  const { id } = req.query;
  if (!id) {
    return res.status(400).json({ error: "Search id is required" });
  }
  try {
    const response = await fetch(
      `https://api.wety.org/cognates/${id}?distLang=2048&descLang=2048`
    );
    const data = await response.json();
    res.status(200).json(data);
  } catch (error) {
    res.status(500).json({ error: "Failed to fetch etymology data" });
  }
}
