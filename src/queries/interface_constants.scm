(program
  (block_comment)*
  (line_comment)*
  (interface_declaration 
      (interface_body
          ((block_comment)*
          (line_comment)*
          (constant_declaration) @constant)* 
      )
	)
)