@echo off
echo Running Bloat Buster...
python bloat_buster.py
echo.
echo Cleaning Rust build artifacts...
cargo clean
cd backend
cargo clean
cd ..
echo.
echo Done! Your disk space should be reclaimed now.
pause
