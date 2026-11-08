// ESLint do cliente da API: TypeScript estrito com tipos, sem `any`, sem armazenamento do navegador.
import js from '@eslint/js';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  { ignores: ['dist/**', 'src/gerado/**'] },
  js.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  {
    languageOptions: {
      parserOptions: { projectService: true, tsconfigRootDir: import.meta.dirname },
    },
    rules: {
      '@typescript-eslint/no-explicit-any': 'error',
      // SEC-CLI-3: nada de token ou dado de saúde no armazenamento do navegador.
      'no-restricted-globals': [
        'error',
        { name: 'localStorage', message: 'Proibido (SEC-CLI-3).' },
        { name: 'sessionStorage', message: 'Proibido (SEC-CLI-3).' },
      ],
    },
  },
  { files: ['**/*.js'], ...tseslint.configs.disableTypeChecked },
);
