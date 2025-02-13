import type { NextApiRequest, NextApiResponse } from "next";
import type { ResponseData, ErrorResponse } from "../../src/types";

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse<ResponseData | ErrorResponse>
) {
  const { id } = req.query;
  if (!id) {
    return res.status(400).json({ error: "Search id is required" });
  }
  try {
    const response = await fetch(
      `https://api.wety.org/cognates/${id}?distLang=2048&descLang=2016&descLang=2204&descLang=6160`
    );
    const data: ResponseData = await response.json();
    res.status(200).json(data);
  } catch (error) {
    res.status(500).json({ error: "Failed to fetch etymology data" });
  }
}
