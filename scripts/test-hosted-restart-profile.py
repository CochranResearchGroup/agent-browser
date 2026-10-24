import importlib.util
from pathlib import Path
import unittest
spec = importlib.util.spec_from_file_location('restart', Path(__file__).with_name('smoke-hosted-browser-restart-native.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
class ProfileAdmission(unittest.TestCase):
    def test_flattened_and_nul_arguments_bind_exact_directory(self):
        for raw in (b'chrome\0--user-data-dir=/private/owned/\0about:blank\0',
                    b'chrome --user-data-dir=/private/owned/ about:blank\0'):
            self.assertTrue(module.exact_profile_argument(raw, '/private/owned'))
            self.assertFalse(module.exact_profile_argument(raw, '/private/own'))
    def test_ambiguous_or_missing_profile_is_refused(self):
        for raw in (b'chrome\0', b'chrome --user-data-dir=/private/owned --user-data-dir=/foreign\0'):
            self.assertFalse(module.exact_profile_argument(raw, '/private/owned'))
if __name__ == '__main__': unittest.main()
