// ESLint do design system: TypeScript estrito com tipos, sem `any`, sem dangerouslySetInnerHTML
// (SEC-CLI-1) e sem armazenamento do navegador (SEC-CLI-3).
import js from '@eslint/js';
import reactHooks from 'eslint-plugin-react-hooks';
import tseslint from 'typescript-eslint';

export default tseslint.config(
  { ignores: ['dist/**'] },
  js.configs.recommended,
  ...tseslint.configs.strictTypeChecked,
  ...tseslint.configs.stylisticTypeChecked,
  {
    plugins: { 'react-hooks': reactHooks },
    languageOptions: {
      parserOptions: { projectService: true, tsconfigRootDir: import.meta.dirname },
    },
    rules: {
      ...reactHooks.configs.recommended.rules,
      '@typescript-eslint/no-explicit-any': 'error',
      // Conflita com no-non-null-assertion (strict): preferimos a asserção explícita de tipo.
      '@typescript-eslint/non-nullable-type-assertion-style': 'off',
      'no-restricted-syntax': [
        'error',
        {
          selector: "JSXAttribute[name.name='dangerouslySetInnerHTML']",
          message: 'Proibido dangerouslySetInnerHTML (SEC-CLI-1): renderize como texto.',
        },
      ],
      'no-restricted-globals': [
        'error',
        { name: 'localStorage', message: 'Proibido (SEC-CLI-3).' },
        { name: 'sessionStorage', message: 'Proibido (SEC-CLI-3).' },
      ],
      'no-restricted-properties': [
        'error',
        { object: 'window', property: 'localStorage', message: 'Proibido (SEC-CLI-3).' },
        { object: 'window', property: 'sessionStorage', message: 'Proibido (SEC-CLI-3).' },
        { object: 'globalThis', property: 'localStorage', message: 'Proibido (SEC-CLI-3).' },
        { object: 'globalThis', property: 'sessionStorage', message: 'Proibido (SEC-CLI-3).' },
      ],
    },
  },
  { files: ['**/*.js'], ...tseslint.configs.disableTypeChecked },
);
