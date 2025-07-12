
import i18n, { BackendModule, ReadCallback } from 'i18next';
import { initReactI18next } from 'react-i18next';
import LanguageDetector from 'i18next-browser-languagedetector';
import { readTextFile } from '@tauri-apps/plugin-fs';
import { resolveResource } from '@tauri-apps/api/path';

// Custom i18next backend to load translations using Tauri's API
const TauriBackend: BackendModule = {
  type: 'backend',
  init() {
    /* nothing to init */
  },
  read(language: string, namespace: string, callback: ReadCallback) {    
    resolveResource(`locales/${language}/${namespace}.json`)
      .then(resourcePath => readTextFile(resourcePath))
      .then(fileContent => {        
        try {
          callback(null, JSON.parse(fileContent));
        } catch (e) {
          callback(e as Error, false);
        }
      })
      .catch((error) => {
        callback(error, false);
      });
  },
};

i18n
  .use(TauriBackend)
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    supportedLngs: ['en', 'zh'],
    fallbackLng: 'en',
    ns: ['translation'],
    defaultNS: 'translation',
    detection: {
      order: ['localStorage', 'navigator'],
      caches: ['localStorage'],
    },
    backend: {},
    react: {
      useSuspense: false,
    },
  });

export default i18n;
