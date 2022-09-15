run:
	set -e ;\
	peers=`echo $(peers) | tr -d '[]'`;\
	if [ -z $$peers ]; then\
		cargo run -- --node $(node);\
	else\
		cargo run -- --node $(node) --peers $$peers;\
	fi
