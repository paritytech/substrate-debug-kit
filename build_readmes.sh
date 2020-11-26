all_dirs=("remote-externalities" "offline-election" "sub-storage" "sub-du")

for d in "${all_dirs[@]}"; do
	if [ -d $d ]; then
		cd $d && cargo readme > README.md && cd ..
	fi
done
