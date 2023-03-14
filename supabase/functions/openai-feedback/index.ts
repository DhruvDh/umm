import { serve } from "std/server";
import {
  ChatCompletionRequestMessage,
  CreateChatCompletionRequest,
} from "openai";
import { createClient } from "@supabase/supabase-js";

const corsHeaders = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers":
    "authorization, x-client-info, apikey, content-type",
};

serve(async (req) => {
  // This is needed if you're planning to invoke your function from a browser.
  if (req.method === "OPTIONS") {
    return new Response("ok", { headers: corsHeaders });
  } else {
    const { prompt_id } = await req.json();

    const supabaseClient = createClient(
      Deno.env.get("SUPABASE_URL") ?? "",
      Deno.env.get("SUPABASE_ANON_KEY") ?? "",
    );

    const { data, error } = await supabaseClient
      .from("prompts")
      .select("*")
      .eq("id", prompt_id)
      .select("messages")
      .single();

    if (error) throw error;
    let messages: Array<ChatCompletionRequestMessage>;
    try {
      messages =
        data.messages as unknown as Array<ChatCompletionRequestMessage> ?? [
          {
            "role": "system",
            "content":
              "- You are an AI teaching assistant at UNC, Charlotte.\n- The course uses a computer program is used to generate a prompt, then the prompt is shared with you.\n- This prompt is helpful explanation of errors or assignment grading feedback.\n- The explanation/feedback is what the user - a Student - is here for.\n- However, something has gone wrong with the program that was to generate this prompt and now you cannot help the student. Please explain this to the student.\n- **Note: The student cannot respond, so do not expect him to**.\n> Respond in markdown syntax only.",
          },
        ];
    } catch (_e) {
      messages = [
        {
          "role": "system",
          "content":
            "- You are an AI teaching assistant at UNC, Charlotte.\n- The course uses a computer program is used to generate a prompt, then the prompt is shared with you.\n- This prompt is helpful explanation of errors or assignment grading feedback.\n- The explanation/feedback is what the user - a Student - is here for.\n- However, something has gone wrong with the program that was to generate this prompt and now you cannot help the student. Please explain this to the student.\n- **Note: The student cannot respond, so do not expect him to**.\n> Respond in markdown syntax only.",
        },
      ];
    }

    const completionConfig: CreateChatCompletionRequest = {
      model: "gpt-3.5-turbo",
      temperature: 0.51,
      top_p: 0.96,
      n: 1,
      frequency_penalty: 0.0,
      presence_penalty: 0.0,
      stream: true,
      messages,
    };

    return fetch("https://api.openai.com/v1/chat/completions", {
      method: "POST",
      headers: {
        Authorization: `Bearer ${Deno.env.get("OPENAI_API_KEY")}`,
        "Content-Type": "application/json",
      },
      body: JSON.stringify(completionConfig),
    });
  }
});
