use crate::prelude::*;

impl GscClient {
    fn json_ls(&self, rpats: &[RemotePattern]) -> Result<()> {
        for rpat in rpats {
            assert!(rpat.name.is_empty(), "not handled");

            let response = self.fetch_raw_file_list(rpat.hw)?;
            let json = response.text()?;
            v1!("{}", json);
        }

        Ok(())
    }

    pub fn ls(&self, rpats: &[RemotePattern]) -> Result<()> {
        if self.config().json_output() {
            return self.json_ls(rpats);
        }

        for rpat in rpats {
            self.try_warn(|| {
                let files = self.fetch_nonempty_matching_file_list(rpat)?;

                if rpats.len() > 1 {
                    v1!("{}:", rpat);
                }

                let mut table = tabular::Table::new("{:>}  {:<}  [{:<}] {:<}");

                for file in &files {
                    table.add_row(
                        tabular::Row::new()
                            .with_cell(file.byte_count.separate_with_commas())
                            .with_cell(&file.upload_time)
                            .with_cell(file.purpose.to_char())
                            .with_cell(&file.name),
                    );
                }

                v1!("{}", table);

                Ok(())
            });
        }

        Ok(())
    }
}
