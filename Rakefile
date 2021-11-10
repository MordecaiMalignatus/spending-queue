task :default => :test

task :release do
  sh('cargo build --release')
  sh('cp ./target/release/sq ~/.local/bin/')
end

task :test do
  sh('cargo test')
end
