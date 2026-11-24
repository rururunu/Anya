/** 是否已经有一条能用来对话的模型凭据。 */
export function hasConfiguredProvider(input: {
  deepseekApiKey?: string;
  customProviders?: { baseUrl?: string; apiKey?: string }[];
}): boolean {
  if (input.deepseekApiKey?.trim()) return true;
  return (input.customProviders ?? []).some(
    (provider) => Boolean(provider.baseUrl?.trim()) && Boolean(provider.apiKey?.trim()),
  );
}
