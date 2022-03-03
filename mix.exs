defmodule SortedSetKv.MixProject do
  use Mix.Project
  @version "0.1.3"
  def project do
    [
      app: :sorted_set_kv,
      version: @version,
      elixir: "~> 1.12",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      docs: docs(),
      package: package(),
      source_url: "https://github.com/SoCal-Software-Labs/SortedSetKV"
    ]
  end

  defp docs() do
    [
      extras: [
        LICENSE: [title: "License"],
        "README.md": [title: "Overview"]
      ],
      main: "readme",
      assets: "assets",
      canonical: "http://hexdocs.pm/sorted_set_kv",
      source_url: "https://github.com/SoCal-Software-Labs/SortedSetKV",
      source_ref: "v#{@version}",
      formatters: ["html"]
    ]
  end

  # Run "mix help compile.app" to learn about applications.
  def application do
    [
      extra_applications: [:logger]
    ]
  end

  defp package() do
    [
      description:
        "An ultrafast double-ended queue, scored sorted set and key value database with an optional secondary u64 index.",
      files: [
        "lib",
        "native/.cargo",
        "native/sortedsetkv/src",
        "native/sortedsetkv/Cargo.toml",
        "LICENSE",
        "mix.exs"
      ],
      maintainers: ["Kyle Hanson"],
      licenses: ["MIT"],
      links: %{
        "GitHub" => "https://github.com/SoCal-Software-Labs/SortedSetKV"
      }
    ]
  end

  # Run "mix help deps" to learn about dependencies.
  defp deps do
    [
      {:rustler, "~> 0.23.0"},
      {:ex_doc, ">= 0.0.0", only: :dev, runtime: false}
    ]
  end
end
