/** Full image reference, without appending a tag when the image name already has one. */
export function imageRef(image: string, tag?: string | null): string {
  const last = image.split("/").pop() ?? image;
  if (image.includes("@") || last.includes(":")) return image;
  return `${image}:${tag || "latest"}`;
}
