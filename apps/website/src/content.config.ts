import { defineCollection } from 'astro:content';
import { docsLoader } from '@astrojs/starlight/loaders';
import { glob } from 'astro/loaders';
import { docsSchema, i18nSchema } from '@astrojs/starlight/schema';
export const collections = {
  i18n: defineCollection({
    loader: glob({ pattern: '**/*.json', base: './src/content/i18n' }),
    schema: i18nSchema(),
  }),
  docs: defineCollection({ loader: docsLoader(), schema: docsSchema() }),
};
