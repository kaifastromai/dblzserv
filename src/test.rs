#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn gen_csv_combos() {
        let header =
            "number_bottom,number_top,gender_bottom,gender_top,#is_red,#is_blue,#is_green,#is_yellow";

        let colors = vec!["red", "blue", "green", "yellow"];
        //generate numbers 1-10 for each color
        let combos = colors
            .iter()
            .flat_map(|color| {
                (0..10)
                    .map(|n| {
                        let n = n + 1;
                        let gender = if n % 2 == 0 { "B" } else { "G" };
                        let is_fields = colors
                            .iter()
                            .map(|c| if c == color { "True" } else { "False" })
                            .collect::<Vec<_>>()
                            .join(",");
                        let number_gender = format!("{n},{n},{gender},{gender}");
                        number_gender + "," + &is_fields
                    })
                    .collect::<Vec<String>>()
            })
            .collect::<Vec<_>>();
        let mut file = std::fs::File::create("combos.csv").unwrap();
        writeln!(file, "{}", header).unwrap();
        let combos = combos.join("\n");
        write!(file, "{}", combos).unwrap();
    }
}
